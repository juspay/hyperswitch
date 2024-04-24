use std::collections::HashMap;

use base64::Engine;
use common_utils::{crypto::GenerateDigest, date_time, pii::Email};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use ring::digest;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self as connector_utils, PaymentsAuthorizeRequestData, RouterData},
    consts,
    core::errors,
    services,
    types::{self, domain, storage::enums},
};

mod auth_error {
    pub const INVALID_SIGNATURE: &str = "INVALID_SIGNATURE";
}
mod zsl_version {
    pub const VERSION_1: &str = "1";
}

pub struct ZslRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for ZslRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, txn_amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = connector_utils::get_amount_as_string(currency_unit, txn_amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

pub struct ZslAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for ZslAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_id: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZslPaymentsRequest {
    process_type: ProcessType,
    process_code: ProcessCode,
    txn_amt: String,
    ccy: api_models::enums::Currency,
    mer_ref: String,
    mer_txn_date: String,
    mer_id: Secret<String>,
    lang: String,
    success_url: String,
    failure_url: String,
    success_s2s_url: String,
    failure_s2s_url: String,
    enctype: EncodingType,
    signature: Secret<String>,
    country: api_models::enums::CountryAlpha2,
    verno: String,
    service_code: ServiceCode,
    cust_tag: String,
    #[serde(flatten)]
    payment_method: ZslPaymentMethods,
    name: Option<Secret<String>>,
    family_name: Option<Secret<String>>,
    tel_phone: Option<Secret<String>>,
    email: Option<Email>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ZslPaymentMethods {
    LocalBankTransfer(LocalBankTransaferRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalBankTransaferRequest {
    bank_code: Option<String>,
    pay_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessType {
    #[serde(rename = "0200")]
    PaymentRequest,
    #[serde(rename = "0208")]
    PaymentResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessCode {
    #[serde(rename = "200002")]
    API,
    #[serde(rename = "200003")]
    CallBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncodingType {
    #[serde(rename = "1")]
    MD5,
    #[serde(rename = "2")]
    Sha1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ServiceCode {
    MPG,
}

impl TryFrom<&ZslRouterData<&types::PaymentsAuthorizeRouterData>> for ZslPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ZslRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method = match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                match *bank_transfer_data {
                    domain::BankTransferData::LocalBankTransfer { bank_code } => Ok(
                        ZslPaymentMethods::LocalBankTransfer(LocalBankTransaferRequest {
                            bank_code,
                            pay_method: None,
                        }),
                    ),
                    domain::BankTransferData::AchBankTransfer { .. }
                    | domain::BankTransferData::SepaBankTransfer { .. }
                    | domain::BankTransferData::BacsBankTransfer { .. }
                    | domain::BankTransferData::MultibancoBankTransfer { .. }
                    | domain::BankTransferData::PermataBankTransfer { .. }
                    | domain::BankTransferData::BcaBankTransfer { .. }
                    | domain::BankTransferData::BniVaBankTransfer { .. }
                    | domain::BankTransferData::BriVaBankTransfer { .. }
                    | domain::BankTransferData::CimbVaBankTransfer { .. }
                    | domain::BankTransferData::DanamonVaBankTransfer { .. }
                    | domain::BankTransferData::MandiriVaBankTransfer { .. }
                    | domain::BankTransferData::Pix {}
                    | domain::BankTransferData::Pse {} => {
                        Err(errors::ConnectorError::NotImplemented(
                            connector_utils::get_unimplemented_payment_method_error_message(
                                item.router_data.connector.as_str(),
                            ),
                        ))
                    }
                }
            }
            domain::PaymentMethodData::Card(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_utils::get_unimplemented_payment_method_error_message(
                        item.router_data.connector.as_str(),
                    ),
                ))
            }
        }?;
        let auth_type = ZslAuthType::try_from(&item.router_data.connector_auth_type)?;
        let key: Secret<String> = auth_type.api_key;
        let mer_id = auth_type.merchant_id;
        let mer_txn_date =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let txn_amt = item.amount.clone();
        let ccy = item.router_data.request.currency;
        let mer_ref = item.router_data.connector_request_reference_id.clone();
        let signature = calculate_signature(
            EncodingType::MD5,
            ZslSignatureType::RequestSignature {
                txn_amt: txn_amt.clone(),
                ccy: ccy.to_string(),
                mer_ref: mer_ref.clone(),
                mer_id: mer_id.clone().expose(),
                mer_txn_date: mer_txn_date.clone(),
                key: key.expose(),
            },
        )?;
        let tel_phone = item.router_data.get_optional_billing_phone_number();
        let email = item.router_data.get_optional_billing_email();
        let name = item.router_data.get_optional_billing_first_name();
        let family_name = item.router_data.get_optional_billing_last_name();
        let router_url = item.router_data.request.get_router_return_url()?;
        let webhook_url = item.router_data.request.get_webhook_url()?;
        let billing_country = item.router_data.get_billing_country()?;

        let lang = item
            .router_data
            .request
            .browser_info
            .as_ref()
            .and_then(|browser_data| {
                browser_data.language.as_ref().map(|language| {
                    language
                        .split_once('-')
                        .map_or(language.to_uppercase(), |(lang, _)| lang.to_uppercase())
                })
            })
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "browser_info.language",
            })?;

        let cust_tag = item
            .router_data
            .customer_id
            .clone()
            .and_then(|customer_id| {
                let cust_id = customer_id.replace(['_', '-'], "");
                let id_len = cust_id.len();
                if id_len > 10 {
                    cust_id.get(id_len - 10..id_len).map(|id| id.to_string())
                } else {
                    Some(cust_id)
                }
            })
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "customer_id",
            })?;

        Ok(Self {
            process_type: ProcessType::PaymentRequest,
            process_code: ProcessCode::API,
            txn_amt,
            ccy,
            mer_ref,
            mer_txn_date,
            mer_id,
            lang,
            success_url: router_url.clone(),
            failure_url: router_url.clone(),
            success_s2s_url: webhook_url.clone(),
            failure_s2s_url: webhook_url.clone(),
            enctype: EncodingType::MD5,
            signature,
            verno: zsl_version::VERSION_1.to_owned(),
            service_code: ServiceCode::MPG,
            country: billing_country,
            payment_method,
            name,
            family_name,
            tel_phone,
            email,
            cust_tag,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZslPaymentsResponse {
    process_type: ProcessType,
    process_code: ProcessCode,
    status: String,
    mer_ref: String,
    mer_id: String,
    enctype: EncodingType,
    txn_url: String,
    signature: Secret<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, ZslPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ZslPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if item.response.status.eq("0") && !item.response.txn_url.is_empty() {
            let auth_type = ZslAuthType::try_from(&item.data.connector_auth_type)?;
            let key: Secret<String> = auth_type.api_key;
            let mer_id = auth_type.merchant_id;
            let calculated_signature = calculate_signature(
                item.response.enctype,
                ZslSignatureType::ResponseSignature {
                    status: item.response.status.clone(),
                    txn_url: item.response.txn_url.clone(),
                    mer_ref: item.response.mer_ref.clone(),
                    mer_id: mer_id.clone().expose(),
                    key: key.expose(),
                },
            )?;

            if calculated_signature.clone().eq(&item.response.signature) {
                let decoded_redirect_url_bytes: Vec<u8> = base64::engine::general_purpose::STANDARD
                    .decode(item.response.txn_url.clone())
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                let redirect_url = String::from_utf8(decoded_redirect_url_bytes)
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                Ok(Self {
                    status: enums::AttemptStatus::AuthenticationPending, // Redirect is always expected after success response
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            item.response.mer_ref.clone(),
                        ),
                        redirection_data: Some(services::RedirectForm::Form {
                            endpoint: redirect_url,
                            method: services::Method::Get,
                            form_fields: HashMap::new(),
                        }),
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(item.response.mer_ref.clone()),
                        incremental_authorization_allowed: None,
                        integrity_object: None,
                    }),
                    ..item.data
                })
            } else {
                // When the signature check fails
                Ok(Self {
                    status: enums::AttemptStatus::Failure,
                    response: Err(types::ErrorResponse {
                        code: consts::NO_ERROR_CODE.to_string(),
                        message: auth_error::INVALID_SIGNATURE.to_string(),
                        reason: Some(auth_error::INVALID_SIGNATURE.to_string()),
                        status_code: item.http_code,
                        attempt_status: Some(enums::AttemptStatus::Failure),
                        connector_transaction_id: Some(item.response.mer_ref.clone()),
                    }),
                    ..item.data
                })
            }
        } else {
            let error_reason =
                ZslResponseStatus::try_from(item.response.status.clone())?.to_string();
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(types::ErrorResponse {
                    code: item.response.status.clone(),
                    message: error_reason.clone(),
                    reason: Some(error_reason.clone()),
                    status_code: item.http_code,
                    attempt_status: Some(enums::AttemptStatus::Failure),
                    connector_transaction_id: Some(item.response.mer_ref.clone()),
                }),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZslWebhookResponse {
    pub process_type: ProcessType,
    pub process_code: ProcessCode,
    pub status: String,
    pub txn_id: String,
    pub txn_date: String,
    pub paid_ccy: api_models::enums::Currency,
    pub paid_amt: String,
    pub consr_paid_ccy: api_models::enums::Currency,
    pub consr_paid_amt: String,
    pub service_fee_ccy: api_models::enums::Currency,
    pub service_fee: String,
    pub txn_amt: String,
    pub ccy: String,
    pub mer_ref: String,
    pub mer_txn_date: String,
    pub mer_id: String,
    pub enctype: EncodingType,
    pub signature: Secret<String>,
}

impl types::transformers::ForeignFrom<String> for api_models::webhooks::IncomingWebhookEvent {
    fn foreign_from(status: String) -> Self {
        match status.as_str() {
            //any response with status != 0 are a failed deposit transaction
            "0" => Self::PaymentIntentSuccess,
            _ => Self::PaymentIntentFailure,
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, ZslWebhookResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ZslWebhookResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if item.response.status == "0" {
            Ok(Self {
                status: enums::AttemptStatus::Charged,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.mer_ref.clone(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.mer_ref.clone()),
                    incremental_authorization_allowed: None,
                    integrity_object: None,
                }),
                ..item.data
            })
        } else {
            let error_reason =
                ZslResponseStatus::try_from(item.response.status.clone())?.to_string();
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(types::ErrorResponse {
                    code: item.response.status.clone(),
                    message: error_reason.clone(),
                    reason: Some(error_reason.clone()),
                    status_code: item.http_code,
                    attempt_status: Some(enums::AttemptStatus::Failure),
                    connector_transaction_id: Some(item.response.mer_ref.clone()),
                }),
                ..item.data
            })
        }
    }
}

impl TryFrom<String> for ZslResponseStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(status: String) -> Result<Self, Self::Error> {
        match status.as_str() {
            "0" => Ok(Self::Normal),
            "1000" => Ok(Self::InternalError),
            "1001" => Ok(Self::BreakDownMessageError),
            "1002" => Ok(Self::FormatError),
            "1004" => Ok(Self::InvalidTransaction),
            "1005" => Ok(Self::TransactionCountryNotFound),
            "1006" => Ok(Self::MerchantIdNotFound),
            "1007" => Ok(Self::AccountDisabled),
            "1008" => Ok(Self::DuplicateMerchantReference),
            "1009" => Ok(Self::InvalidPayAmount),
            "1010" => Ok(Self::PayAmountNotFound),
            "1011" => Ok(Self::InvalidCurrencyCode),
            "1012" => Ok(Self::CurrencyCodeNotFound),
            "1013" => Ok(Self::ReferenceNotFound),
            "1014" => Ok(Self::TransmissionTimeNotFound),
            "1015" => Ok(Self::PayMethodNotFound),
            "1016" => Ok(Self::BankCodeNotFound),
            "1017" => Ok(Self::InvalidShowPayPage),
            "1018" => Ok(Self::ShowPayPageNotFound),
            "1019" => Ok(Self::SuccessUrlNotFound),
            "1020" => Ok(Self::SuccessCallbackUrlNotFound),
            "1021" => Ok(Self::FailUrlNotFound),
            "1022" => Ok(Self::FailCallbackUrlNotFound),
            "1023" => Ok(Self::MacNotFound),
            "1025" => Ok(Self::OriginalTransactionNotFound),
            "1026" => Ok(Self::DeblockDataError),
            "1028" => Ok(Self::PspAckNotYetReturn),
            "1029" => Ok(Self::BankBranchNameNotFound),
            "1030" => Ok(Self::BankAccountIDNotFound),
            "1031" => Ok(Self::BankAccountNameNotFound),
            "1032" => Ok(Self::IdentityIDNotFound),
            "1033" => Ok(Self::ErrorConnectingToPsp),
            "1034" => Ok(Self::CountryPspNotAvailable),
            "1035" => Ok(Self::UnsupportedPayAmount),
            "1036" => Ok(Self::RecordMismatch),
            "1037" => Ok(Self::NoRecord),
            "1038" => Ok(Self::PspError),
            "1039" => Ok(Self::UnsupportedEncryptionType),
            "1040" => Ok(Self::ExceedTransactionLimitCount),
            "1041" => Ok(Self::ExceedTransactionLimitAmount),
            "1042" => Ok(Self::ExceedTransactionAccountLimitCount),
            "1043" => Ok(Self::ExceedTransactionAccountLimitAmount),
            "1044" => Ok(Self::ExchangeRateError),
            "1045" => Ok(Self::InvalidEncoding),
            "1046" => Ok(Self::CustomerNameNotFound),
            "1047" => Ok(Self::CustomerFamilyNameNotFound),
            "1048" => Ok(Self::CustomerTelPhoneNotFound),
            "1049" => Ok(Self::InsufficientFund),
            "1050" => Ok(Self::ServiceCodeIsMissing),
            "1051" => Ok(Self::CurrencyIdNotMatch),
            "1052" => Ok(Self::NoPendingRecord),
            "1053" => Ok(Self::NoLoadBalancerRuleDefineForTransaction),
            "1054" => Ok(Self::NoPaymentProviderAvailable),
            "1055" => Ok(Self::UnsupportedPayMethod),
            "1056" => Ok(Self::PendingTransaction),
            "1057" => Ok(Self::OtherError1059),
            "1058" => Ok(Self::OtherError1058),
            "1059" => Ok(Self::OtherError1059),
            "1084" => Ok(Self::InvalidRequestId),
            "5043" => Ok(Self::BeneficiaryBankAccountIsNotAvailable),
            "5053" => Ok(Self::BaidNotFound),
            "5057" => Ok(Self::InvalidBaid),
            "5059" => Ok(Self::InvalidBaidStatus),
            "5107" => Ok(Self::AutoUploadBankDisabled),
            "5108" => Ok(Self::InvalidNature),
            "5109" => Ok(Self::SmsCreateDateNotFound),
            "5110" => Ok(Self::InvalidSmsCreateDate),
            "5111" => Ok(Self::RecordNotFound),
            "5112" => Ok(Self::InsufficientBaidAvailableBalance),
            "5113" => Ok(Self::ExceedTxnAmountLimit),
            "5114" => Ok(Self::BaidBalanceNotFound),
            "5115" => Ok(Self::AutoUploadIndicatorNotFound),
            "5116" => Ok(Self::InvalidBankAcctStatus),
            "5117" => Ok(Self::InvalidAutoUploadIndicator),
            "5118" => Ok(Self::InvalidPidStatus),
            "5119" => Ok(Self::InvalidProviderStatus),
            "5120" => Ok(Self::InvalidBankAccountSystemSwitchEnabled),
            "5121" => Ok(Self::AutoUploadProviderDisabled),
            "5122" => Ok(Self::AutoUploadBankNotFound),
            "5123" => Ok(Self::AutoUploadBankAcctNotFound),
            "5124" => Ok(Self::AutoUploadProviderNotFound),
            "5125" => Ok(Self::UnsupportedBankCode),
            "5126" => Ok(Self::BalanceOverrideIndicatorNotFound),
            "5127" => Ok(Self::InvalidBalanceOverrideIndicator),
            "10000" => Ok(Self::VernoInvalid),
            "10001" => Ok(Self::ServiceCodeInvalid),
            "10002" => Ok(Self::PspResponseSignatureIsNotValid),
            "10003" => Ok(Self::ProcessTypeNotFound),
            "10004" => Ok(Self::ProcessCodeNotFound),
            "10005" => Ok(Self::EnctypeNotFound),
            "10006" => Ok(Self::VernoNotFound),
            "10007" => Ok(Self::DepositBankNotFound),
            "10008" => Ok(Self::DepositFlowNotFound),
            "10009" => Ok(Self::CustDepositDateNotFound),
            "10010" => Ok(Self::CustTagNotFound),
            "10011" => Ok(Self::CountryValueInvalid),
            "10012" => Ok(Self::CurrencyCodeValueInvalid),
            "10013" => Ok(Self::MerTxnDateInvalid),
            "10014" => Ok(Self::CustDepositDateInvalid),
            "10015" => Ok(Self::TxnAmtInvalid),
            "10016" => Ok(Self::SuccessCallbackUrlInvalid),
            "10017" => Ok(Self::DepositFlowInvalid),
            "10018" => Ok(Self::ProcessTypeInvalid),
            "10019" => Ok(Self::ProcessCodeInvalid),
            "10020" => Ok(Self::UnsupportedMerRefLength),
            "10021" => Ok(Self::DepositBankLengthOverLimit),
            "10022" => Ok(Self::CustTagLengthOverLimit),
            "10023" => Ok(Self::SignatureLengthOverLimit),
            "10024" => Ok(Self::RequestContainInvalidTag),
            "10025" => Ok(Self::RequestSignatureNotMatch),
            "10026" => Ok(Self::InvalidCustomer),
            "10027" => Ok(Self::SchemeNotFound),
            "10028" => Ok(Self::PspResponseFieldsMissing),
            "10029" => Ok(Self::PspResponseMerRefNotMatchWithRequestMerRef),
            "10030" => Ok(Self::PspResponseMerIdNotMatchWithRequestMerId),
            "10031" => Ok(Self::UpdateDepositFailAfterResponse),
            "10032" => Ok(Self::UpdateUsedLimitTransactionCountFailAfterSuccessResponse),
            "10033" => Ok(Self::UpdateCustomerLastDepositRecordAfterSuccessResponse),
            "10034" => Ok(Self::CreateDepositFail),
            "10035" => Ok(Self::CreateDepositMsgFail),
            "10036" => Ok(Self::UpdateStatusSubStatusFail),
            "10037" => Ok(Self::AddDepositRecordToSchemeAccount),
            "10038" => Ok(Self::EmptyResponse),
            "10039" => Ok(Self::AubConfirmErrorFromPh),
            "10040" => Ok(Self::ProviderEmailAddressNotFound),
            "10041" => Ok(Self::AubConnectionTimeout),
            "10042" => Ok(Self::AubConnectionIssue),
            "10043" => Ok(Self::AubMsgTypeMissing),
            "10044" => Ok(Self::AubMsgCodeMissing),
            "10045" => Ok(Self::AubVersionMissing),
            "10046" => Ok(Self::AubEncTypeMissing),
            "10047" => Ok(Self::AubSignMissing),
            "10048" => Ok(Self::AubInfoMissing),
            "10049" => Ok(Self::AubErrorCodeMissing),
            "10050" => Ok(Self::AubMsgTypeInvalid),
            "10051" => Ok(Self::AubMsgCodeInvalid),
            "10052" => Ok(Self::AubBaidMissing),
            "10053" => Ok(Self::AubResponseSignNotMatch),
            "10054" => Ok(Self::SmsConnectionTimeout),
            "10055" => Ok(Self::SmsConnectionIssue),
            "10056" => Ok(Self::SmsConfirmErrorFromPh),
            "10057" => Ok(Self::SmsMsgTypeMissing),
            "10058" => Ok(Self::SmsMsgCodeMissing),
            "10059" => Ok(Self::SmsVersionMissing),
            "10060" => Ok(Self::SmsEncTypeMissing),
            "10061" => Ok(Self::SmsSignMissing),
            "10062" => Ok(Self::SmsInfoMissing),
            "10063" => Ok(Self::SmsErrorCodeMissing),
            "10064" => Ok(Self::SmsMsgTypeInvalid),
            "10065" => Ok(Self::SmsMsgCodeInvalid),
            "10066" => Ok(Self::SmsResponseSignNotMatch),
            "10067" => Ok(Self::SmsRequestReachMaximumLimit),
            "10068" => Ok(Self::SyncConnectionTimeout),
            "10069" => Ok(Self::SyncConnectionIssue),
            "10070" => Ok(Self::SyncConfirmErrorFromPh),
            "10071" => Ok(Self::SyncMsgTypeMissing),
            "10072" => Ok(Self::SyncMsgCodeMissing),
            "10073" => Ok(Self::SyncVersionMissing),
            "10074" => Ok(Self::SyncEncTypeMissing),
            "10075" => Ok(Self::SyncSignMissing),
            "10076" => Ok(Self::SyncInfoMissing),
            "10077" => Ok(Self::SyncErrorCodeMissing),
            "10078" => Ok(Self::SyncMsgTypeInvalid),
            "10079" => Ok(Self::SyncMsgCodeInvalid),
            "10080" => Ok(Self::SyncResponseSignNotMatch),
            "10081" => Ok(Self::AccountExpired),
            "10082" => Ok(Self::ExceedMaxMinAmount),
            "10083" => Ok(Self::WholeNumberAmountLessThanOne),
            "10084" => Ok(Self::AddDepositRecordToSchemeChannel),
            "10085" => Ok(Self::UpdateUtilizedAmountFailAfterSuccessResponse),
            "10086" => Ok(Self::PidResponseInvalidFormat),
            "10087" => Ok(Self::PspNameNotFound),
            "10088" => Ok(Self::LangIsMissing),
            "10089" => Ok(Self::FailureCallbackUrlInvalid),
            "10090" => Ok(Self::SuccessRedirectUrlInvalid),
            "10091" => Ok(Self::FailureRedirectUrlInvalid),
            "10092" => Ok(Self::LangValueInvalid),
            "10093" => Ok(Self::OnlineDepositSessionTimeout),
            "10094" => Ok(Self::AccessPaymentPageRouteFieldMissing),
            "10095" => Ok(Self::AmountNotMatch),
            "10096" => Ok(Self::PidCallbackFieldsMissing),
            "10097" => Ok(Self::TokenNotMatch),
            "10098" => Ok(Self::OperationDuplicated),
            "10099" => Ok(Self::PayPageDomainNotAvailable),
            "10100" => Ok(Self::PayPageConfirmSignatureNotMatch),
            "10101" => Ok(Self::PaymentPageConfirmationFieldMissing),
            "10102" => Ok(Self::MultipleCallbackFromPsp),
            "10103" => Ok(Self::PidNotAvailable),
            "10104" => Ok(Self::PidDepositUrlNotValidOrEmp),
            "10105" => Ok(Self::PspSelfRedirectTagNotValid),
            "20000" => Ok(Self::InternalError20000),
            "20001" => Ok(Self::DepositTimeout),
            _ => Err(errors::ConnectorError::ResponseHandlingFailed.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ZslResponseStatus {
    Normal,
    InternalError,
    BreakDownMessageError,
    FormatError,
    InvalidTransaction,
    TransactionCountryNotFound,
    MerchantIdNotFound,
    AccountDisabled,
    DuplicateMerchantReference,
    InvalidPayAmount,
    PayAmountNotFound,
    InvalidCurrencyCode,
    CurrencyCodeNotFound,
    ReferenceNotFound,
    TransmissionTimeNotFound,
    PayMethodNotFound,
    BankCodeNotFound,
    InvalidShowPayPage,
    ShowPayPageNotFound,
    SuccessUrlNotFound,
    SuccessCallbackUrlNotFound,
    FailUrlNotFound,
    FailCallbackUrlNotFound,
    MacNotFound,
    OriginalTransactionNotFound,
    DeblockDataError,
    PspAckNotYetReturn,
    BankBranchNameNotFound,
    BankAccountIDNotFound,
    BankAccountNameNotFound,
    IdentityIDNotFound,
    ErrorConnectingToPsp,
    CountryPspNotAvailable,
    UnsupportedPayAmount,
    RecordMismatch,
    NoRecord,
    PspError,
    UnsupportedEncryptionType,
    ExceedTransactionLimitCount,
    ExceedTransactionLimitAmount,
    ExceedTransactionAccountLimitCount,
    ExceedTransactionAccountLimitAmount,
    ExchangeRateError,
    InvalidEncoding,
    CustomerNameNotFound,
    CustomerFamilyNameNotFound,
    CustomerTelPhoneNotFound,
    InsufficientFund,
    ServiceCodeIsMissing,
    CurrencyIdNotMatch,
    NoPendingRecord,
    NoLoadBalancerRuleDefineForTransaction,
    NoPaymentProviderAvailable,
    UnsupportedPayMethod,
    PendingTransaction,
    OtherError1059,
    OtherError1058,
    InvalidRequestId,
    BeneficiaryBankAccountIsNotAvailable,
    BaidNotFound,
    InvalidBaid,
    InvalidBaidStatus,
    AutoUploadBankDisabled,
    InvalidNature,
    SmsCreateDateNotFound,
    InvalidSmsCreateDate,
    RecordNotFound,
    InsufficientBaidAvailableBalance,
    ExceedTxnAmountLimit,
    BaidBalanceNotFound,
    AutoUploadIndicatorNotFound,
    InvalidBankAcctStatus,
    InvalidAutoUploadIndicator,
    InvalidPidStatus,
    InvalidProviderStatus,
    InvalidBankAccountSystemSwitchEnabled,
    AutoUploadProviderDisabled,
    AutoUploadBankNotFound,
    AutoUploadBankAcctNotFound,
    AutoUploadProviderNotFound,
    UnsupportedBankCode,
    BalanceOverrideIndicatorNotFound,
    InvalidBalanceOverrideIndicator,
    VernoInvalid,
    ServiceCodeInvalid,
    PspResponseSignatureIsNotValid,
    ProcessTypeNotFound,
    ProcessCodeNotFound,
    EnctypeNotFound,
    VernoNotFound,
    DepositBankNotFound,
    DepositFlowNotFound,
    CustDepositDateNotFound,
    CustTagNotFound,
    CountryValueInvalid,
    CurrencyCodeValueInvalid,
    MerTxnDateInvalid,
    CustDepositDateInvalid,
    TxnAmtInvalid,
    SuccessCallbackUrlInvalid,
    DepositFlowInvalid,
    ProcessTypeInvalid,
    ProcessCodeInvalid,
    UnsupportedMerRefLength,
    DepositBankLengthOverLimit,
    CustTagLengthOverLimit,
    SignatureLengthOverLimit,
    RequestContainInvalidTag,
    RequestSignatureNotMatch,
    InvalidCustomer,
    SchemeNotFound,
    PspResponseFieldsMissing,
    PspResponseMerRefNotMatchWithRequestMerRef,
    PspResponseMerIdNotMatchWithRequestMerId,
    UpdateDepositFailAfterResponse,
    UpdateUsedLimitTransactionCountFailAfterSuccessResponse,
    UpdateCustomerLastDepositRecordAfterSuccessResponse,
    CreateDepositFail,
    CreateDepositMsgFail,
    UpdateStatusSubStatusFail,
    AddDepositRecordToSchemeAccount,
    EmptyResponse,
    AubConfirmErrorFromPh,
    ProviderEmailAddressNotFound,
    AubConnectionTimeout,
    AubConnectionIssue,
    AubMsgTypeMissing,
    AubMsgCodeMissing,
    AubVersionMissing,
    AubEncTypeMissing,
    AubSignMissing,
    AubInfoMissing,
    AubErrorCodeMissing,
    AubMsgTypeInvalid,
    AubMsgCodeInvalid,
    AubBaidMissing,
    AubResponseSignNotMatch,
    SmsConnectionTimeout,
    SmsConnectionIssue,
    SmsConfirmErrorFromPh,
    SmsMsgTypeMissing,
    SmsMsgCodeMissing,
    SmsVersionMissing,
    SmsEncTypeMissing,
    SmsSignMissing,
    SmsInfoMissing,
    SmsErrorCodeMissing,
    SmsMsgTypeInvalid,
    SmsMsgCodeInvalid,
    SmsResponseSignNotMatch,
    SmsRequestReachMaximumLimit,
    SyncConnectionTimeout,
    SyncConnectionIssue,
    SyncConfirmErrorFromPh,
    SyncMsgTypeMissing,
    SyncMsgCodeMissing,
    SyncVersionMissing,
    SyncEncTypeMissing,
    SyncSignMissing,
    SyncInfoMissing,
    SyncErrorCodeMissing,
    SyncMsgTypeInvalid,
    SyncMsgCodeInvalid,
    SyncResponseSignNotMatch,
    AccountExpired,
    ExceedMaxMinAmount,
    WholeNumberAmountLessThanOne,
    AddDepositRecordToSchemeChannel,
    UpdateUtilizedAmountFailAfterSuccessResponse,
    PidResponseInvalidFormat,
    PspNameNotFound,
    LangIsMissing,
    FailureCallbackUrlInvalid,
    SuccessRedirectUrlInvalid,
    FailureRedirectUrlInvalid,
    LangValueInvalid,
    OnlineDepositSessionTimeout,
    AccessPaymentPageRouteFieldMissing,
    AmountNotMatch,
    PidCallbackFieldsMissing,
    TokenNotMatch,
    OperationDuplicated,
    PayPageDomainNotAvailable,
    PayPageConfirmSignatureNotMatch,
    PaymentPageConfirmationFieldMissing,
    MultipleCallbackFromPsp,
    PidNotAvailable,
    PidDepositUrlNotValidOrEmp,
    PspSelfRedirectTagNotValid,
    InternalError20000,
    DepositTimeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZslErrorResponse {
    pub status: String,
}

pub enum ZslSignatureType {
    RequestSignature {
        txn_amt: String,
        ccy: String,
        mer_ref: String,
        mer_id: String,
        mer_txn_date: String,
        key: String,
    },
    ResponseSignature {
        status: String,
        txn_url: String,
        mer_ref: String,
        mer_id: String,
        key: String,
    },
    WebhookSignature {
        status: String,
        txn_id: String,
        txn_date: String,
        paid_ccy: String,
        paid_amt: String,
        mer_ref: String,
        mer_id: String,
        key: String,
    },
}

pub fn calculate_signature(
    enctype: EncodingType,
    signature_data: ZslSignatureType,
) -> Result<Secret<String>, error_stack::Report<errors::ConnectorError>> {
    let signature_data = match signature_data {
        ZslSignatureType::RequestSignature {
            txn_amt,
            ccy,
            mer_ref,
            mer_id,
            mer_txn_date,
            key,
        } => format!("{txn_amt}{ccy}{mer_ref}{mer_id}{mer_txn_date}{key}"),
        ZslSignatureType::ResponseSignature {
            status,
            txn_url,
            mer_ref,
            mer_id,
            key,
        } => {
            format!("{status}{txn_url}{mer_ref}{mer_id}{key}")
        }
        ZslSignatureType::WebhookSignature {
            status,
            txn_id,
            txn_date,
            paid_ccy,
            paid_amt,
            mer_ref,
            mer_id,
            key,
        } => format!("{status}{txn_id}{txn_date}{paid_ccy}{paid_amt}{mer_ref}{mer_id}{key}"),
    };
    let message = signature_data.as_bytes();

    let encoded_data = match enctype {
        EncodingType::MD5 => hex::encode(
            common_utils::crypto::Md5
                .generate_digest(message)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
        ),
        EncodingType::Sha1 => {
            hex::encode(digest::digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, message))
        }
    };
    Ok(Secret::new(encoded_data))
}
