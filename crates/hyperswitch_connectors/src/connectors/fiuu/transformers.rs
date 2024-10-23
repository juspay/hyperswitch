use std::collections::HashMap;

use api_models::payments;
use cards::CardNumber;
use common_enums::{enums, BankNames, CaptureMethod, Currency};
use common_utils::{
    crypto::GenerateDigest,
    errors::CustomResult,
    ext_traits::Encode,
    request::Method,
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{
        BankRedirectData, Card, GooglePayWalletData, PaymentMethodData, RealTimePaymentData,
        WalletData,
    },
    router_data::{
        ApplePayPredecryptData, ConnectorAuthType, ErrorResponse, PaymentMethodToken, RouterData,
    },
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use strum::Display;
use url::Url;

// These needs to be accepted from SDK, need to be done after 1.0.0 stability as API contract will change
const GOOGLEPAY_API_VERSION_MINOR: u8 = 0;
const GOOGLEPAY_API_VERSION: u8 = 2;

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        PaymentsSyncResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    unimplemented_payment_method,
    utils::{
        self, ApplePayDecrypt, PaymentsAuthorizeRequestData, QrImage, RefundsRequestData,
        RouterData as _,
    },
};

pub struct FiuuRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for FiuuRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct FiuuAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) verify_key: Secret<String>,
    pub(super) secret_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for FiuuAuthType {
    type Error = Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                merchant_id: key1.to_owned(),
                verify_key: api_key.to_owned(),
                secret_key: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum TxnType {
    Sals,
    Auts,
}

impl TryFrom<Option<CaptureMethod>> for TxnType {
    type Error = Report<errors::ConnectorError>;
    fn try_from(capture_method: Option<CaptureMethod>) -> Result<Self, Self::Error> {
        match capture_method {
            Some(CaptureMethod::Automatic) => Ok(Self::Sals),
            Some(CaptureMethod::Manual) => Ok(Self::Auts),
            _ => Err(errors::ConnectorError::CaptureMethodNotSupported.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Display, Debug, Clone)]
enum TxnChannel {
    #[serde(rename = "CREDITAN")]
    #[strum(serialize = "CREDITAN")]
    Creditan,
    #[serde(rename = "RPP_DUITNOWQR")]
    #[strum(serialize = "RPP_DUITNOWQR")]
    RppDuitNowQr,
}

#[derive(Serialize, Deserialize, Display, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum FPXTxnChannel {
    FpxAbb,
    FpxUob,
    FpxAbmb,
    FpxScb,
    FpxBsn,
    FpxKfh,
    FpxBmmb,
    FpxBkrm,
    FpxHsbc,
    FpxAgrobank,
    FpxBocm,
    FpxMb2u,
    FpxCimbclicks,
    FpxAmb,
    FpxHlb,
    FpxPbb,
    FpxRhb,
    FpxBimb,
    FpxOcbc,
}
impl TryFrom<BankNames> for FPXTxnChannel {
    type Error = Report<errors::ConnectorError>;
    fn try_from(bank_names: BankNames) -> Result<Self, Self::Error> {
        match bank_names {
            BankNames::AffinBank => Ok(Self::FpxAbb),
            BankNames::AgroBank => Ok(Self::FpxAgrobank),
            BankNames::AllianceBank => Ok(Self::FpxAbmb),
            BankNames::AmBank => Ok(Self::FpxAmb),
            BankNames::BankOfChina => Ok(Self::FpxBocm),
            BankNames::BankIslam => Ok(Self::FpxBimb),
            BankNames::BankMuamalat => Ok(Self::FpxBmmb),
            BankNames::BankRakyat => Ok(Self::FpxBkrm),
            BankNames::BankSimpananNasional => Ok(Self::FpxBsn),
            BankNames::CimbBank => Ok(Self::FpxCimbclicks),
            BankNames::HongLeongBank => Ok(Self::FpxHlb),
            BankNames::HsbcBank => Ok(Self::FpxHsbc),
            BankNames::KuwaitFinanceHouse => Ok(Self::FpxKfh),
            BankNames::Maybank => Ok(Self::FpxMb2u),
            BankNames::PublicBank => Ok(Self::FpxPbb),
            BankNames::RhbBank => Ok(Self::FpxRhb),
            BankNames::StandardCharteredBank => Ok(Self::FpxScb),
            BankNames::UobBank => Ok(Self::FpxUob),
            BankNames::OcbcBank => Ok(Self::FpxOcbc),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Fiuu"),
            ))?,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuPaymentRequest {
    #[serde(rename = "MerchantID")]
    merchant_id: Secret<String>,
    reference_no: String,
    txn_type: TxnType,
    txn_currency: Currency,
    txn_amount: StringMajorUnit,
    signature: Secret<String>,
    #[serde(rename = "ReturnURL")]
    return_url: Option<String>,
    #[serde(flatten)]
    payment_method_data: FiuuPaymentMethodData,
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum FiuuPaymentMethodData {
    FiuuQRData(Box<FiuuQRData>),
    FiuuCardData(Box<FiuuCardData>),
    FiuuFpxData(Box<FiuuFPXData>),
    FiuuGooglePayData(Box<FiuuGooglePayData>),
    FiuuApplePayData(Box<FiuuApplePayData>),
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuFPXData {
    #[serde(rename = "non_3DS")]
    non_3ds: i32,
    txn_channel: FPXTxnChannel,
}
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuQRData {
    txn_channel: TxnChannel,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct FiuuCardData {
    #[serde(rename = "non_3DS")]
    non_3ds: i32,
    #[serde(rename = "TxnChannel")]
    txn_channel: TxnChannel,
    cc_pan: CardNumber,
    cc_cvv2: Secret<String>,
    cc_month: Secret<String>,
    cc_year: Secret<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct FiuuApplePayData {
    #[serde(rename = "TxnChannel")]
    txn_channel: TxnChannel,
    cc_month: Secret<String>,
    cc_year: Secret<String>,
    cc_token: Secret<String>,
    eci: Option<String>,
    token_cryptogram: Secret<String>,
    token_type: FiuuTokenType,
    #[serde(rename = "non_3DS")]
    non_3ds: i32,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum FiuuTokenType {
    ApplePay,
    GooglePay,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuGooglePayData {
    txn_channel: TxnChannel,
    #[serde(rename = "GooglePay[apiVersion]")]
    api_version: u8,
    #[serde(rename = "GooglePay[apiVersionMinor]")]
    api_version_minor: u8,
    #[serde(rename = "GooglePay[paymentMethodData][info][assuranceDetails][accountVerified]")]
    account_verified: Option<bool>,
    #[serde(
        rename = "GooglePay[paymentMethodData][info][assuranceDetails][cardHolderAuthenticated]"
    )]
    card_holder_authenticated: Option<bool>,
    #[serde(rename = "GooglePay[paymentMethodData][info][cardDetails]")]
    card_details: String,
    #[serde(rename = "GooglePay[paymentMethodData][info][cardNetwork]")]
    card_network: String,
    #[serde(rename = "GooglePay[paymentMethodData][tokenizationData][token]")]
    token: Secret<String>,
    #[serde(rename = "GooglePay[paymentMethodData][tokenizationData][type]")]
    tokenization_data_type: Secret<String>,
    #[serde(rename = "GooglePay[paymentMethodData][type]")]
    pm_type: String,
    #[serde(rename = "SCREAMING_SNAKE_CASE")]
    token_type: FiuuTokenType,
    #[serde(rename = "non_3DS")]
    non_3ds: i32,
}

pub fn calculate_signature(
    signature_data: String,
) -> Result<Secret<String>, Report<errors::ConnectorError>> {
    let message = signature_data.as_bytes();
    let encoded_data = hex::encode(
        common_utils::crypto::Md5
            .generate_digest(message)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?,
    );
    Ok(Secret::new(encoded_data))
}

impl TryFrom<&FiuuRouterData<&PaymentsAuthorizeRouterData>> for FiuuPaymentRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &FiuuRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let txn_currency = item.router_data.request.currency;
        let txn_amount = item.amount.clone();
        let reference_no = item.router_data.connector_request_reference_id.clone();
        let verify_key = auth.verify_key.peek().to_string();
        let signature = calculate_signature(format!(
            "{}{merchant_id}{reference_no}{verify_key}",
            txn_amount.get_amount_as_string()
        ))?;
        let txn_type = match item.router_data.request.is_auto_capture()? {
            true => TxnType::Sals,
            false => TxnType::Auts,
        };
        let return_url = item.router_data.request.router_return_url.clone();
        let non_3ds = match item.router_data.is_three_ds() {
            false => 1,
            true => 0,
        };
        let payment_method_data = match item.router_data.request.payment_method_data {
            PaymentMethodData::Card(ref card) => FiuuPaymentMethodData::try_from((card, &non_3ds)),
            PaymentMethodData::RealTimePayment(ref real_time_payment_data) => {
                match *real_time_payment_data.clone() {
                    RealTimePaymentData::DuitNow {} => {
                        Ok(FiuuPaymentMethodData::FiuuQRData(Box::new(FiuuQRData {
                            txn_channel: TxnChannel::RppDuitNowQr,
                        })))
                    }
                    RealTimePaymentData::Fps {}
                    | RealTimePaymentData::PromptPay {}
                    | RealTimePaymentData::VietQr {} => {
                        Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("fiuu"),
                        )
                        .into())
                    }
                }
            }
            PaymentMethodData::BankRedirect(ref bank_redirect_data) => match bank_redirect_data {
                BankRedirectData::OnlineBankingFpx { ref issuer } => {
                    Ok(FiuuPaymentMethodData::FiuuFpxData(Box::new(FiuuFPXData {
                        txn_channel: FPXTxnChannel::try_from(*issuer)?,
                        non_3ds,
                    })))
                }
                BankRedirectData::BancontactCard { .. }
                | BankRedirectData::Bizum {}
                | BankRedirectData::Blik { .. }
                | BankRedirectData::Eps { .. }
                | BankRedirectData::Giropay { .. }
                | BankRedirectData::Ideal { .. }
                | BankRedirectData::Interac { .. }
                | BankRedirectData::OnlineBankingCzechRepublic { .. }
                | BankRedirectData::OnlineBankingFinland { .. }
                | BankRedirectData::OnlineBankingPoland { .. }
                | BankRedirectData::OnlineBankingSlovakia { .. }
                | BankRedirectData::OpenBankingUk { .. }
                | BankRedirectData::Przelewy24 { .. }
                | BankRedirectData::Sofort { .. }
                | BankRedirectData::Trustly { .. }
                | BankRedirectData::OnlineBankingThailand { .. }
                | BankRedirectData::LocalBankRedirect {} => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("fiuu"),
                    )
                    .into())
                }
            },
            PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                WalletData::GooglePay(google_pay_data) => {
                    FiuuPaymentMethodData::try_from(google_pay_data)
                }
                WalletData::ApplePay(_apple_pay_data) => {
                    let payment_method_token = item.router_data.get_payment_method_token()?;
                    match payment_method_token {
                        PaymentMethodToken::Token(_) => {
                            Err(unimplemented_payment_method!("Apple Pay", "Manual", "Fiuu"))?
                        }
                        PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                            FiuuPaymentMethodData::try_from(decrypt_data)
                        }
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Fiuu"))?
                        }
                    }
                }
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::MomoRedirect(_)
                | WalletData::KakaoPayRedirect(_)
                | WalletData::GoPayRedirect(_)
                | WalletData::GcashRedirect(_)
                | WalletData::ApplePayRedirect(_)
                | WalletData::ApplePayThirdPartySdk(_)
                | WalletData::DanaRedirect {}
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
                    utils::get_unimplemented_payment_method_error_message("fiuu"),
                )
                .into()),
            },
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("fiuu"),
                )
                .into())
            }
        }?;

        Ok(Self {
            merchant_id: auth.merchant_id,
            reference_no,
            txn_type,
            txn_currency,
            txn_amount,
            return_url,
            payment_method_data,
            signature,
        })
    }
}

impl TryFrom<(&Card, &i32)> for FiuuPaymentMethodData {
    type Error = Report<errors::ConnectorError>;
    fn try_from((req_card, non_3ds): (&Card, &i32)) -> Result<Self, Self::Error> {
        Ok(Self::FiuuCardData(Box::new(FiuuCardData {
            txn_channel: TxnChannel::Creditan,
            non_3ds: *non_3ds,
            cc_pan: req_card.card_number.clone(),
            cc_cvv2: req_card.card_cvc.clone(),
            cc_month: req_card.card_exp_month.clone(),
            cc_year: req_card.card_exp_year.clone(),
        })))
    }
}

impl TryFrom<&GooglePayWalletData> for FiuuPaymentMethodData {
    type Error = Report<errors::ConnectorError>;
    fn try_from(data: &GooglePayWalletData) -> Result<Self, Self::Error> {
        Ok(Self::FiuuGooglePayData(Box::new(FiuuGooglePayData {
            txn_channel: TxnChannel::Creditan,
            api_version: GOOGLEPAY_API_VERSION,
            api_version_minor: GOOGLEPAY_API_VERSION_MINOR,
            account_verified: data
                .info
                .assurance_details
                .as_ref()
                .map(|details| details.account_verified),
            card_holder_authenticated: data
                .info
                .assurance_details
                .as_ref()
                .map(|details| details.card_holder_authenticated),
            card_details: data.info.card_details.clone(),
            card_network: data.info.card_network.clone(),
            token: data.tokenization_data.token.clone().into(),
            tokenization_data_type: data.tokenization_data.token_type.clone().into(),
            pm_type: data.pm_type.clone(),
            token_type: FiuuTokenType::GooglePay,
            // non_3ds field Applicable to card processing via specific processor using specific currency for pre-approved partner only.
            // Equal to 0 by default and 1 for non-3DS transaction, That is why it is hardcoded to 1 for googlepay transactions.
            non_3ds: 1,
        })))
    }
}

impl TryFrom<Box<ApplePayPredecryptData>> for FiuuPaymentMethodData {
    type Error = Report<errors::ConnectorError>;
    fn try_from(decrypt_data: Box<ApplePayPredecryptData>) -> Result<Self, Self::Error> {
        Ok(Self::FiuuApplePayData(Box::new(FiuuApplePayData {
            txn_channel: TxnChannel::Creditan,
            cc_month: decrypt_data.get_expiry_month()?,
            cc_year: decrypt_data.get_four_digit_expiry_year()?,
            cc_token: decrypt_data.application_primary_account_number,
            eci: decrypt_data.payment_data.eci_indicator,
            token_cryptogram: decrypt_data.payment_data.online_payment_cryptogram,
            token_type: FiuuTokenType::ApplePay,
            // non_3ds field Applicable to card processing via specific processor using specific currency for pre-approved partner only.
            // Equal to 0 by default and 1 for non-3DS transaction, That is why it is hardcoded to 1 for apple pay decrypt flow transactions.
            non_3ds: 1,
        })))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentsResponse {
    pub reference_no: String,
    #[serde(rename = "TxnID")]
    pub txn_id: String,
    pub txn_type: TxnType,
    pub txn_currency: Currency,
    pub txn_amount: StringMajorUnit,
    pub txn_channel: String,
    pub txn_data: TxnData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DuitNowQrCodeResponse {
    pub reference_no: String,
    pub txn_type: TxnType,
    pub txn_currency: Currency,
    pub txn_amount: StringMajorUnit,
    pub txn_channel: String,
    #[serde(rename = "TxnID")]
    pub txn_id: String,
    pub txn_data: QrTxnData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QrTxnData {
    pub request_data: QrRequestData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QrRequestData {
    pub qr_data: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FiuuPaymentsResponse {
    PaymentResponse(Box<PaymentsResponse>),
    QRPaymentResponse(Box<DuitNowQrCodeResponse>),
    Error(FiuuErrorResponse),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TxnData {
    #[serde(rename = "RequestURL")]
    pub request_url: String,
    pub request_type: RequestType,
    pub request_data: RequestData,
    pub request_method: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RequestType {
    Redirect,
    Response,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestData {
    NonThreeDS(NonThreeDSResponseData),
    RedirectData(Option<HashMap<String, String>>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QrCodeData {
    #[serde(rename = "tranID")]
    pub tran_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonThreeDSResponseData {
    #[serde(rename = "tranID")]
    pub tran_id: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_desc: Option<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, FiuuPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            FiuuPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            FiuuPaymentsResponse::QRPaymentResponse(ref response) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(response.txn_id.clone()),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: get_qr_metadata(response)?,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            FiuuPaymentsResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_desc.clone(),
                    reason: Some(error.error_desc),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
            FiuuPaymentsResponse::PaymentResponse(data) => match data.txn_data.request_data {
                RequestData::RedirectData(redirection_data) => {
                    let redirection_data = Some(RedirectForm::Form {
                        endpoint: data.txn_data.request_url.to_string(),
                        method: if data.txn_data.request_method.as_str() == "POST" {
                            Method::Post
                        } else {
                            Method::Get
                        },
                        form_fields: redirection_data.unwrap_or_default(),
                    });
                    Ok(Self {
                        status: enums::AttemptStatus::AuthenticationPending,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(data.txn_id),
                            redirection_data,
                            mandate_reference: None,
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: None,
                            incremental_authorization_allowed: None,
                            charge_id: None,
                        }),
                        ..item.data
                    })
                }
                RequestData::NonThreeDS(non_threeds_data) => {
                    let status = match non_threeds_data.status.as_str() {
                        "00" => {
                            if item.data.request.is_auto_capture()? {
                                Ok(enums::AttemptStatus::Charged)
                            } else {
                                Ok(enums::AttemptStatus::Authorized)
                            }
                        }
                        "11" => Ok(enums::AttemptStatus::Failure),
                        "22" => Ok(enums::AttemptStatus::Pending),
                        other => Err(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from(other.to_owned()),
                        )),
                    }?;
                    let response = if status == enums::AttemptStatus::Failure {
                        Err(ErrorResponse {
                            code: non_threeds_data
                                .error_code
                                .clone()
                                .unwrap_or_else(|| "NO_ERROR_CODE".to_string()),
                            message: non_threeds_data
                                .error_desc
                                .clone()
                                .unwrap_or_else(|| "NO_ERROR_MESSAGE".to_string()),
                            reason: non_threeds_data.error_desc.clone(),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                        })
                    } else {
                        Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(data.txn_id),
                            redirection_data: None,
                            mandate_reference: None,
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: None,
                            incremental_authorization_allowed: None,
                            charge_id: None,
                        })
                    };
                    Ok(Self {
                        status,
                        response,
                        ..item.data
                    })
                }
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuRefundRequest {
    pub refund_type: RefundType,
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "RefID")]
    pub ref_id: String,
    #[serde(rename = "TxnID")]
    pub txn_id: String,
    pub amount: StringMajorUnit,
    pub signature: Secret<String>,
    #[serde(rename = "notify_url")]
    pub notify_url: Option<Url>,
}
#[derive(Debug, Serialize, Display)]
pub enum RefundType {
    #[serde(rename = "P")]
    #[strum(serialize = "P")]
    Partial,
}

impl TryFrom<&FiuuRouterData<&RefundsRouterData<Execute>>> for FiuuRefundRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &FiuuRouterData<&RefundsRouterData<Execute>>) -> Result<Self, Self::Error> {
        let auth: FiuuAuthType = FiuuAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let txn_amount = item.amount.clone();
        let reference_no = item.router_data.connector_request_reference_id.clone();
        let txn_id = item.router_data.request.connector_transaction_id.clone();
        let secret_key = auth.secret_key.peek().to_string();
        Ok(Self {
            refund_type: RefundType::Partial,
            merchant_id: auth.merchant_id,
            ref_id: reference_no.clone(),
            txn_id: txn_id.clone(),
            amount: txn_amount.clone(),
            signature: calculate_signature(format!(
                "{}{merchant_id}{reference_no}{txn_id}{}{secret_key}",
                RefundType::Partial,
                txn_amount.get_amount_as_string()
            ))?,
            notify_url: Some(
                Url::parse(&item.router_data.request.get_webhook_url()?)
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            ),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuRefundSuccessResponse {
    #[serde(rename = "RefundID")]
    refund_id: i64,
    status: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FiuuRefundResponse {
    Success(FiuuRefundSuccessResponse),
    Error(FiuuErrorResponse),
}
impl TryFrom<RefundsResponseRouterData<Execute, FiuuRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, FiuuRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            FiuuRefundResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_desc.clone(),
                    reason: Some(error.error_desc),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
            FiuuRefundResponse::Success(refund_data) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: refund_data.refund_id.to_string(),
                    refund_status: match refund_data.status.as_str() {
                        "00" => Ok(enums::RefundStatus::Success),
                        "11" => Ok(enums::RefundStatus::Failure),
                        "22" => Ok(enums::RefundStatus::Pending),
                        other => Err(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from(other.to_owned()),
                        )),
                    }?,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FiuuErrorResponse {
    pub error_code: String,
    pub error_desc: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FiuuPaymentSyncRequest {
    amount: StringMajorUnit,
    #[serde(rename = "txID")]
    tx_id: String,
    domain: String,
    skey: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum FiuuPaymentResponse {
    FiuuPaymentSyncResponse(FiuuPaymentSyncResponse),
    FiuuWebhooksPaymentResponse(FiuuWebhooksPaymentResponse),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuPaymentSyncResponse {
    stat_code: StatCode,
    stat_name: StatName,
    #[serde(rename = "TranID")]
    tran_id: String,
    error_code: String,
    error_desc: String,
    #[serde(rename = "miscellaneous")]
    miscellaneous: Option<HashMap<String, Secret<String>>>,
}

#[derive(Debug, Serialize, Deserialize, Display, Clone, PartialEq)]
pub enum StatCode {
    #[serde(rename = "00")]
    Success,
    #[serde(rename = "11")]
    Failure,
    #[serde(rename = "22")]
    Pending,
}

#[derive(Debug, Serialize, Deserialize, Display, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StatName {
    Captured,
    Settled,
    Authorized,
    Failed,
    Cancelled,
    Chargeback,
    Release,
    #[serde(rename = "reject/hold")]
    RejectHold,
    Blocked,
    #[serde(rename = "ReqCancel")]
    ReqCancel,
    #[serde(rename = "ReqChargeback")]
    ReqChargeback,
    #[serde(rename = "Pending")]
    Pending,
    #[serde(rename = "Unknown")]
    Unknown,
}
impl TryFrom<&PaymentsSyncRouterData> for FiuuPaymentSyncRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.connector_auth_type)?;
        let txn_id = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let verify_key = auth.verify_key.peek().to_string();
        let amount = StringMajorUnitForConnector
            .convert(item.request.amount, item.request.currency)
            .change_context(errors::ConnectorError::AmountConversionFailed)?;
        Ok(Self {
            amount: amount.clone(),
            tx_id: txn_id.clone(),
            domain: merchant_id.clone(),
            skey: calculate_signature(format!(
                "{txn_id}{merchant_id}{verify_key}{}",
                amount.get_amount_as_string()
            ))?,
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<FiuuPaymentResponse>> for PaymentsSyncRouterData {
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<FiuuPaymentResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            FiuuPaymentResponse::FiuuPaymentSyncResponse(response) => {
                let stat_name = response.stat_name;
                let stat_code = response.stat_code.clone();
                let status = enums::AttemptStatus::try_from(FiuuSyncStatus {
                    stat_name,
                    stat_code,
                })?;
                let error_response = if status == enums::AttemptStatus::Failure {
                    Some(ErrorResponse {
                        status_code: item.http_code,
                        code: response.stat_code.to_string(),
                        message: response.stat_name.clone().to_string(),
                        reason: Some(response.stat_name.clone().to_string()),
                        attempt_status: Some(enums::AttemptStatus::Failure),
                        connector_transaction_id: None,
                    })
                } else {
                    None
                };
                let payments_response_data = PaymentsResponseData::TransactionResponse {
                    resource_id: item.data.request.connector_transaction_id.clone(),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                };
                Ok(Self {
                    status,
                    response: error_response.map_or_else(|| Ok(payments_response_data), Err),
                    ..item.data
                })
            }
            FiuuPaymentResponse::FiuuWebhooksPaymentResponse(response) => {
                let status = enums::AttemptStatus::try_from(FiuuWebhookStatus {
                    capture_method: item.data.request.capture_method,
                    status: response.status,
                })?;
                let error_response = if status == enums::AttemptStatus::Failure {
                    Some(ErrorResponse {
                        status_code: item.http_code,
                        code: response
                            .error_code
                            .clone()
                            .unwrap_or(consts::NO_ERROR_CODE.to_owned()),
                        message: response
                            .error_code
                            .clone()
                            .unwrap_or(consts::NO_ERROR_MESSAGE.to_owned()),
                        reason: response.error_desc.clone(),
                        attempt_status: Some(enums::AttemptStatus::Failure),
                        connector_transaction_id: None,
                    })
                } else {
                    None
                };
                let payments_response_data = PaymentsResponseData::TransactionResponse {
                    resource_id: item.data.request.connector_transaction_id.clone(),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                };
                Ok(Self {
                    status,
                    response: error_response.map_or_else(|| Ok(payments_response_data), Err),
                    ..item.data
                })
            }
        }
    }
}

pub struct FiuuWebhookStatus {
    pub capture_method: Option<CaptureMethod>,
    pub status: FiuuPaymentWebhookStatus,
}

impl TryFrom<FiuuWebhookStatus> for enums::AttemptStatus {
    type Error = Report<errors::ConnectorError>;
    fn try_from(webhook_status: FiuuWebhookStatus) -> Result<Self, Self::Error> {
        match webhook_status.status {
            FiuuPaymentWebhookStatus::Success => match webhook_status.capture_method {
                Some(CaptureMethod::Automatic) => Ok(Self::Charged),
                Some(CaptureMethod::Manual) => Ok(Self::Authorized),
                _ => Err(errors::ConnectorError::UnexpectedResponseError(
                    bytes::Bytes::from(webhook_status.status.to_string()),
                ))?,
            },
            FiuuPaymentWebhookStatus::Failure => Ok(Self::Failure),
            FiuuPaymentWebhookStatus::Pending => Ok(Self::AuthenticationPending),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentCaptureRequest {
    domain: String,
    #[serde(rename = "tranID")]
    tran_id: String,
    amount: StringMajorUnit,
    #[serde(rename = "RefID")]
    ref_id: String,
    skey: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentCaptureResponse {
    #[serde(rename = "TranID")]
    tran_id: String,
    stat_code: String,
}

pub struct FiuuSyncStatus {
    pub stat_name: StatName,
    pub stat_code: StatCode,
}

impl TryFrom<FiuuSyncStatus> for enums::AttemptStatus {
    type Error = errors::ConnectorError;
    fn try_from(sync_status: FiuuSyncStatus) -> Result<Self, Self::Error> {
        match (sync_status.stat_code, sync_status.stat_name) {
            (StatCode::Success, StatName::Captured | StatName::Settled) => Ok(Self::Charged), // For Success as StatCode we can only expect Captured,Settled and Authorized as StatName.
            (StatCode::Success, StatName::Authorized) => Ok(Self::Authorized),
            (StatCode::Pending, StatName::Pending) => Ok(Self::AuthenticationPending), // For Pending as StatCode we can only expect Pending and Unknow as StatName.
            (StatCode::Pending, StatName::Unknown) => Ok(Self::Pending),
            (StatCode::Failure, StatName::Cancelled) | (StatCode::Failure, StatName::ReqCancel) => {
                Ok(Self::Voided)
            }
            (StatCode::Failure, _) => Ok(Self::Failure),
            (other, _) => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(other.to_string()),
            )),
        }
    }
}

impl TryFrom<&FiuuRouterData<&PaymentsCaptureRouterData>> for PaymentCaptureRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &FiuuRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.router_data.connector_auth_type)?;
        let merchant_id = auth.merchant_id.peek().to_string();
        let amount = item.amount.clone();
        let txn_id = item.router_data.request.connector_transaction_id.clone();
        let verify_key = auth.verify_key.peek().to_string();
        let signature = calculate_signature(format!(
            "{txn_id}{}{merchant_id}{verify_key}",
            amount.get_amount_as_string()
        ))?;
        Ok(Self {
            domain: merchant_id,
            tran_id: txn_id,
            amount,
            ref_id: item.router_data.connector_request_reference_id.clone(),
            skey: signature,
        })
    }
}
fn capture_status_codes() -> HashMap<&'static str, &'static str> {
    [
        ("00", "Capture successful"),
        ("11", "Capture failed"),
        ("12", "Invalid or unmatched security hash string"),
        ("13", "Not a credit card transaction"),
        ("15", "Requested day is on settlement day"),
        ("16", "Forbidden transaction"),
        ("17", "Transaction not found"),
        ("18", "Missing required parameter"),
        ("19", "Domain not found"),
        ("20", "Temporary out of service"),
        ("21", "Authorization expired"),
        ("23", "Partial capture not allowed"),
        ("24", "Transaction already captured"),
        ("25", "Requested amount exceeds available capture amount"),
        ("99", "General error (contact payment gateway support)"),
    ]
    .into_iter()
    .collect()
}

impl TryFrom<PaymentsCaptureResponseRouterData<PaymentCaptureResponse>>
    for PaymentsCaptureRouterData
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<PaymentCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let status_code = item.response.stat_code;

        let status = match status_code.as_str() {
            "00" => Ok(enums::AttemptStatus::Charged),
            "22" => Ok(enums::AttemptStatus::Pending),
            "11" | "12" | "13" | "15" | "16" | "17" | "18" | "19" | "20" | "21" | "23" | "24"
            | "25" | "99" => Ok(enums::AttemptStatus::Failure),
            other => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(other.to_owned()),
            )),
        }?;
        let capture_message_status = capture_status_codes();
        let error_response = if status == enums::AttemptStatus::Failure {
            Some(ErrorResponse {
                status_code: item.http_code,
                code: status_code.to_owned(),
                message: capture_message_status
                    .get(status_code.as_str())
                    .unwrap_or(&"NO_ERROR_MESSAGE")
                    .to_string(),
                reason: Some(
                    capture_message_status
                        .get(status_code.as_str())
                        .unwrap_or(&"NO_ERROR_REASON")
                        .to_string(),
                ),
                attempt_status: None,
                connector_transaction_id: None,
            })
        } else {
            None
        };
        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.tran_id.to_string()),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charge_id: None,
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FiuuPaymentCancelRequest {
    #[serde(rename = "txnID")]
    txn_id: String,
    domain: String,
    skey: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuPaymentCancelResponse {
    #[serde(rename = "TranID")]
    tran_id: String,
    stat_code: String,
    #[serde(rename = "miscellaneous")]
    miscellaneous: Option<HashMap<String, Secret<String>>>,
}

impl TryFrom<&PaymentsCancelRouterData> for FiuuPaymentCancelRequest {
    type Error = Report<errors::ConnectorError>;

    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.connector_auth_type)?;
        let txn_id = item.request.connector_transaction_id.clone();
        let merchant_id = auth.merchant_id.peek().to_string();
        let secret_key = auth.secret_key.peek().to_string();
        Ok(Self {
            txn_id: txn_id.clone(),
            domain: merchant_id.clone(),
            skey: calculate_signature(format!("{txn_id}{merchant_id}{secret_key}"))?,
        })
    }
}

fn void_status_codes() -> HashMap<&'static str, &'static str> {
    [
        ("00", "Success (will proceed the request)"),
        ("11", "Failure"),
        ("12", "Invalid or unmatched security hash string"),
        ("13", "Not a refundable transaction"),
        ("14", "Transaction date more than 180 days"),
        ("15", "Requested day is on settlement day"),
        ("16", "Forbidden transaction"),
        ("17", "Transaction not found"),
        ("18", "Duplicate partial refund request"),
        ("19", "Merchant not found"),
        ("20", "Missing required parameter"),
        (
            "21",
            "Transaction must be in authorized/captured/settled status",
        ),
    ]
    .into_iter()
    .collect()
}
impl TryFrom<PaymentsCancelResponseRouterData<FiuuPaymentCancelResponse>>
    for PaymentsCancelRouterData
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<FiuuPaymentCancelResponse>,
    ) -> Result<Self, Self::Error> {
        let status_code = item.response.stat_code;
        let status = match status_code.as_str() {
            "00" => Ok(enums::AttemptStatus::Voided),
            "11" | "12" | "13" | "14" | "15" | "16" | "17" | "18" | "19" | "20" | "21" => {
                Ok(enums::AttemptStatus::VoidFailed)
            }
            other => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(other.to_owned()),
            )),
        }?;
        let void_message_status = void_status_codes();
        let error_response = if status == enums::AttemptStatus::VoidFailed {
            Some(ErrorResponse {
                status_code: item.http_code,
                code: status_code.to_owned(),
                message: void_message_status
                    .get(status_code.as_str())
                    .unwrap_or(&"NO_ERROR_MESSAGE")
                    .to_string(),
                reason: Some(
                    void_message_status
                        .get(status_code.as_str())
                        .unwrap_or(&"NO_ERROR_REASON")
                        .to_string(),
                ),
                attempt_status: None,
                connector_transaction_id: None,
            })
        } else {
            None
        };
        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.tran_id.to_string()),
            redirection_data: None,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charge_id: None,
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuRefundSyncRequest {
    #[serde(rename = "TxnID")]
    txn_id: String,
    #[serde(rename = "MerchantID")]
    merchant_id: Secret<String>,
    signature: Secret<String>,
}

impl TryFrom<&RefundSyncRouterData> for FiuuRefundSyncRequest {
    type Error = Report<errors::ConnectorError>;

    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = FiuuAuthType::try_from(&item.connector_auth_type)?;
        let (txn_id, merchant_id, verify_key) = (
            item.request.connector_transaction_id.clone(),
            auth.merchant_id.peek().to_string(),
            auth.verify_key.peek().to_string(),
        );
        let signature = calculate_signature(format!("{txn_id}{merchant_id}{verify_key}"))?;
        Ok(Self {
            txn_id,
            merchant_id: auth.merchant_id,
            signature,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FiuuRefundSyncResponse {
    Success(Vec<RefundData>),
    Error(FiuuErrorResponse),
    Webhook(FiuuWebhooksRefundResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundData {
    #[serde(rename = "RefundID")]
    refund_id: String,
    status: RefundStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    Success,
    Pending,
    Rejected,
    Processing,
}

impl TryFrom<RefundsResponseRouterData<RSync, FiuuRefundSyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = Report<errors::ConnectorError>;

    fn try_from(
        item: RefundsResponseRouterData<RSync, FiuuRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            FiuuRefundSyncResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.error_code.clone(),
                    message: error.error_desc.clone(),
                    reason: Some(error.error_desc),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            }),
            FiuuRefundSyncResponse::Success(refund_data) => {
                let refund = refund_data
                    .iter()
                    .find(|refund| {
                        Some(refund.refund_id.clone()) == item.data.request.connector_refund_id
                    })
                    .ok_or_else(|| errors::ConnectorError::MissingConnectorRefundID)?;
                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: refund.refund_id.clone(),
                        refund_status: enums::RefundStatus::from(refund.status.clone()),
                    }),
                    ..item.data
                })
            }
            FiuuRefundSyncResponse::Webhook(fiuu_webhooks_refund_response) => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: fiuu_webhooks_refund_response.refund_id,
                    refund_status: enums::RefundStatus::from(
                        fiuu_webhooks_refund_response.status.clone(),
                    ),
                }),
                ..item.data
            }),
        }
    }
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Pending => Self::Pending,
            RefundStatus::Success => Self::Success,
            RefundStatus::Rejected => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

pub fn get_qr_metadata(
    response: &DuitNowQrCodeResponse,
) -> CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let image_data = QrImage::new_from_data(response.txn_data.request_data.qr_data.peek().clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str()).ok();
    let display_to_timestamp = None;

    if let Some(image_data_url) = image_data_url {
        let qr_code_info = payments::QrCodeInformation::QrDataUrl {
            image_data_url,
            display_to_timestamp,
        };

        Some(qr_code_info.encode_to_value())
            .transpose()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
    } else {
        Ok(None)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum FiuuWebhooksResponse {
    FiuuWebhookPaymentResponse(FiuuWebhooksPaymentResponse),
    FiuuWebhookRefundResponse(FiuuWebhooksRefundResponse),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FiuuWebhooksPaymentResponse {
    pub skey: Secret<String>,
    pub status: FiuuPaymentWebhookStatus,
    #[serde(rename = "orderid")]
    pub order_id: String,
    #[serde(rename = "tranID")]
    pub tran_id: String,
    pub nbcb: String,
    pub amount: StringMajorUnit,
    pub currency: String,
    pub domain: Secret<String>,
    pub appcode: Secret<String>,
    pub paydate: String,
    pub channel: String,
    pub error_desc: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct FiuuWebhooksRefundResponse {
    pub refund_type: FiuuWebhooksRefundType,
    #[serde(rename = "MerchantID")]
    pub merchant_id: Secret<String>,
    #[serde(rename = "RefID")]
    pub ref_id: String,
    #[serde(rename = "RefundID")]
    pub refund_id: String,
    #[serde(rename = "TxnID")]
    pub txn_id: String,
    pub amount: StringMajorUnit,
    pub status: FiuuRefundsWebhookStatus,
    pub signature: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, strum::Display)]
pub enum FiuuRefundsWebhookStatus {
    #[strum(serialize = "00")]
    #[serde(rename = "00")]
    RefundSuccess,
    #[strum(serialize = "11")]
    #[serde(rename = "11")]
    RefundFailure,
    #[strum(serialize = "22")]
    #[serde(rename = "22")]
    RefundPending,
}

#[derive(Debug, Deserialize, Serialize, Clone, strum::Display)]
pub enum FiuuWebhooksRefundType {
    P,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FiuuWebhookSignauture {
    pub skey: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FiuuWebhookResourceId {
    pub skey: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FiuWebhookEvent {
    pub status: FiuuPaymentWebhookStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone, strum::Display)]
pub enum FiuuPaymentWebhookStatus {
    #[strum(serialize = "00")]
    #[serde(rename = "00")]
    Success,
    #[strum(serialize = "11")]
    #[serde(rename = "11")]
    Failure,
    #[strum(serialize = "22")]
    #[serde(rename = "22")]
    Pending,
}

impl From<FiuuPaymentWebhookStatus> for StatCode {
    fn from(value: FiuuPaymentWebhookStatus) -> Self {
        match value {
            FiuuPaymentWebhookStatus::Success => Self::Success,
            FiuuPaymentWebhookStatus::Failure => Self::Failure,
            FiuuPaymentWebhookStatus::Pending => Self::Pending,
        }
    }
}

impl From<FiuuPaymentWebhookStatus> for api_models::webhooks::IncomingWebhookEvent {
    fn from(value: FiuuPaymentWebhookStatus) -> Self {
        match value {
            FiuuPaymentWebhookStatus::Success => Self::PaymentIntentSuccess,
            FiuuPaymentWebhookStatus::Failure => Self::PaymentIntentFailure,
            FiuuPaymentWebhookStatus::Pending => Self::PaymentIntentProcessing,
        }
    }
}

impl From<FiuuRefundsWebhookStatus> for api_models::webhooks::IncomingWebhookEvent {
    fn from(value: FiuuRefundsWebhookStatus) -> Self {
        match value {
            FiuuRefundsWebhookStatus::RefundSuccess => Self::RefundSuccess,
            FiuuRefundsWebhookStatus::RefundFailure => Self::RefundFailure,
            FiuuRefundsWebhookStatus::RefundPending => Self::EventNotSupported,
        }
    }
}

impl From<FiuuRefundsWebhookStatus> for enums::RefundStatus {
    fn from(value: FiuuRefundsWebhookStatus) -> Self {
        match value {
            FiuuRefundsWebhookStatus::RefundFailure => Self::Failure,
            FiuuRefundsWebhookStatus::RefundSuccess => Self::Success,
            FiuuRefundsWebhookStatus::RefundPending => Self::Pending,
        }
    }
}
