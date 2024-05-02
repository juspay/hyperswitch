use common_utils::ext_traits::Encode;
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, RouterData},
    consts,
    core::errors,
    types::{self, domain, storage::enums},
};
type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct GlobepayPaymentsRequest {
    price: i64,
    description: String,
    currency: enums::Currency,
    channel: GlobepayChannel,
}

#[derive(Debug, Serialize)]
pub enum GlobepayChannel {
    Alipay,
    Wechat,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for GlobepayPaymentsRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let channel: GlobepayChannel = match &item.request.payment_method_data {
            domain::PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                domain::WalletData::AliPayQr(_) => GlobepayChannel::Alipay,
                domain::WalletData::WeChatPayQr(_) => GlobepayChannel::Wechat,
                domain::WalletData::AliPayRedirect(_)
                | domain::WalletData::AliPayHkRedirect(_)
                | domain::WalletData::MomoRedirect(_)
                | domain::WalletData::KakaoPayRedirect(_)
                | domain::WalletData::GoPayRedirect(_)
                | domain::WalletData::GcashRedirect(_)
                | domain::WalletData::ApplePay(_)
                | domain::WalletData::ApplePayRedirect(_)
                | domain::WalletData::ApplePayThirdPartySdk(_)
                | domain::WalletData::DanaRedirect {}
                | domain::WalletData::GooglePay(_)
                | domain::WalletData::GooglePayRedirect(_)
                | domain::WalletData::GooglePayThirdPartySdk(_)
                | domain::WalletData::MbWayRedirect(_)
                | domain::WalletData::MobilePayRedirect(_)
                | domain::WalletData::PaypalRedirect(_)
                | domain::WalletData::PaypalSdk(_)
                | domain::WalletData::SamsungPay(_)
                | domain::WalletData::TwintRedirect {}
                | domain::WalletData::VippsRedirect {}
                | domain::WalletData::TouchNGoRedirect(_)
                | domain::WalletData::WeChatPayRedirect(_)
                | domain::WalletData::CashappQr(_)
                | domain::WalletData::SwishQr(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("globepay"),
                ))?,
            },
            domain::PaymentMethodData::Card(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("globepay"),
                ))?
            }
        };
        let description = item.get_description()?;
        Ok(Self {
            price: item.request.amount,
            description,
            currency: item.request.currency,
            channel,
        })
    }
}

pub struct GlobepayAuthType {
    pub(super) partner_code: Secret<String>,
    pub(super) credential_code: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for GlobepayAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                partner_code: api_key.to_owned(),
                credential_code: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobepayPaymentStatus {
    Success,
    Exists,
}

impl From<GlobepayPaymentStatus> for enums::AttemptStatus {
    fn from(item: GlobepayPaymentStatus) -> Self {
        match item {
            GlobepayPaymentStatus::Success => Self::AuthenticationPending, // this connector only have redirection flows so "Success" is mapped to authenticatoin pending ,ref = "https://pay.globepay.co/docs/en/#api-QRCode-NewQRCode"
            GlobepayPaymentStatus::Exists => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobepayConnectorMetadata {
    image_data_url: url::Url,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobepayPaymentsResponse {
    result_code: Option<GlobepayPaymentStatus>,
    order_id: Option<String>,
    qrcode_img: Option<url::Url>,
    return_code: GlobepayReturnCode, //Execution result
    return_msg: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, strum::Display, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobepayReturnCode {
    Success,
    OrderNotExist,
    OrderMismatch,
    Systemerror,
    InvalidShortId,
    SignTimeout,
    InvalidSign,
    ParamInvalid,
    NotPermitted,
    InvalidChannel,
    DuplicateOrderId,
    OrderNotPaid,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, GlobepayPaymentsResponse, T, types::PaymentsResponseData>>
    for hyperswitch_domain_models::router_data::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            GlobepayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        if item.response.return_code == GlobepayReturnCode::Success {
            let globepay_metadata = GlobepayConnectorMetadata {
                image_data_url: item
                    .response
                    .qrcode_img
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?,
            };
            let connector_metadata = Some(globepay_metadata.encode_to_value())
                .transpose()
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
            let globepay_status = item
                .response
                .result_code
                .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

            Ok(Self {
                status: enums::AttemptStatus::from(globepay_status),
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response
                            .order_id
                            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::Failure, //As this connector gives 200 in failed scenarios . if return_code is not success status is mapped to failure. ref = "https://pay.globepay.co/docs/en/#api-QRCode-NewQRCode"
                response: Err(get_error_response(
                    item.response.return_code,
                    item.response.return_msg,
                    item.http_code,
                )),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobepaySyncResponse {
    pub result_code: Option<GlobepayPaymentPsyncStatus>,
    pub order_id: Option<String>,
    pub return_code: GlobepayReturnCode,
    pub return_msg: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobepayPaymentPsyncStatus {
    Paying,
    CreateFail,
    Closed,
    PayFail,
    PaySuccess,
}

impl From<GlobepayPaymentPsyncStatus> for enums::AttemptStatus {
    fn from(item: GlobepayPaymentPsyncStatus) -> Self {
        match item {
            GlobepayPaymentPsyncStatus::PaySuccess => Self::Charged,
            GlobepayPaymentPsyncStatus::PayFail
            | GlobepayPaymentPsyncStatus::CreateFail
            | GlobepayPaymentPsyncStatus::Closed => Self::Failure,
            GlobepayPaymentPsyncStatus::Paying => Self::AuthenticationPending,
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, GlobepaySyncResponse, T, types::PaymentsResponseData>>
    for hyperswitch_domain_models::router_data::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<F, GlobepaySyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if item.response.return_code == GlobepayReturnCode::Success {
            let globepay_status = item
                .response
                .result_code
                .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
            let globepay_id = item
                .response
                .order_id
                .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
            Ok(Self {
                status: enums::AttemptStatus::from(globepay_status),
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(globepay_id),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::Failure, //As this connector gives 200 in failed scenarios . if return_code is not success status is mapped to failure. ref = "https://pay.globepay.co/docs/en/#api-QRCode-NewQRCode"
                response: Err(get_error_response(
                    item.response.return_code,
                    item.response.return_msg,
                    item.http_code,
                )),
                ..item.data
            })
        }
    }
}

fn get_error_response(
    return_code: GlobepayReturnCode,
    return_msg: Option<String>,
    status_code: u16,
) -> types::ErrorResponse {
    types::ErrorResponse {
        code: return_code.to_string(),
        message: consts::NO_ERROR_MESSAGE.to_string(),
        reason: return_msg,
        status_code,
        attempt_status: None,
        connector_transaction_id: None,
    }
}

#[derive(Debug, Serialize)]
pub struct GlobepayRefundRequest {
    pub fee: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for GlobepayRefundRequest {
    type Error = Error;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            fee: item.request.refund_amount,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GlobepayRefundStatus {
    Waiting,
    CreateFailed,
    Failed,
    Success,
    Finished,
    Change,
}

impl From<GlobepayRefundStatus> for enums::RefundStatus {
    fn from(item: GlobepayRefundStatus) -> Self {
        match item {
            GlobepayRefundStatus::Finished => Self::Success, //FINISHED: Refund success(funds has already been returned to user's account)
            GlobepayRefundStatus::Failed
            | GlobepayRefundStatus::CreateFailed
            | GlobepayRefundStatus::Change => Self::Failure, //CHANGE: Refund can not return to user's account. Manual operation is required
            GlobepayRefundStatus::Waiting | GlobepayRefundStatus::Success => Self::Pending, // SUCCESS: Submission succeeded, but refund is not yet complete. Waiting = Submission succeeded, but refund is not yet complete.
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobepayRefundResponse {
    pub result_code: Option<GlobepayRefundStatus>,
    pub refund_id: Option<String>,
    pub return_code: GlobepayReturnCode,
    pub return_msg: Option<String>,
}

impl<T> TryFrom<types::RefundsResponseRouterData<T, GlobepayRefundResponse>>
    for types::RefundsRouterData<T>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<T, GlobepayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        if item.response.return_code == GlobepayReturnCode::Success {
            let globepay_refund_id = item
                .response
                .refund_id
                .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
            let globepay_refund_status = item
                .response
                .result_code
                .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
            Ok(Self {
                response: Ok(types::RefundsResponseData {
                    connector_refund_id: globepay_refund_id,
                    refund_status: enums::RefundStatus::from(globepay_refund_status),
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                response: Err(get_error_response(
                    item.response.return_code,
                    item.response.return_msg,
                    item.http_code,
                )),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobepayErrorResponse {
    pub return_msg: String,
    pub return_code: GlobepayReturnCode,
    pub message: String,
}
