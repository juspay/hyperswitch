use std::collections::HashMap;

use base64::Engine;
use common_utils::{crypto::GenerateDigest, date_time, pii::Email};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use ring::digest;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self as connector_utils},
    consts,
    core::errors,
    services,
    types::{self, domain, storage::enums},
};

pub const ZSL_VERSION: &str = "1";

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
    opt_1: Option<String>,
    opt_2: Option<String>,
    opt_3: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ProcessType {
    #[serde(rename = "0200")]
    PaymentRequest,
    #[serde(rename = "0208")]
    PaymentResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
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
                    api_models::payments::BankTransferData::LocalBankTransfer { bank_code } => Ok(
                        ZslPaymentMethods::LocalBankTransfer(LocalBankTransaferRequest {
                            bank_code,
                            pay_method: None,
                        }),
                    ),
                    api_models::payments::BankTransferData::AchBankTransfer { .. }
                    | api_models::payments::BankTransferData::SepaBankTransfer { .. }
                    | api_models::payments::BankTransferData::BacsBankTransfer { .. }
                    | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
                    | api_models::payments::BankTransferData::PermataBankTransfer { .. }
                    | api_models::payments::BankTransferData::BcaBankTransfer { .. }
                    | api_models::payments::BankTransferData::BniVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::BriVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::CimbVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::DanamonVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::MandiriVaBankTransfer { .. }
                    | api_models::payments::BankTransferData::Pix {}
                    | api_models::payments::BankTransferData::Pse {} => {
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
                .change_context(errors::ConnectorError::RequestEncodingFailed.into())?;
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
        let billing_data = item.router_data.address.get_payment_billing();
        let billing_address = billing_data
            .as_ref()
            .and_then(|billing_data| billing_data.address.clone());
        let tel_phone = billing_data.and_then(|billing_data| {
            billing_data
                .phone
                .as_ref()
                .and_then(|phone_data| phone_data.number.clone())
        });
        let email = billing_data
            .as_ref()
            .and_then(|billing_data| billing_data.email.clone());
        let billing_country = billing_address
            .as_ref()
            .and_then(|billing_address| billing_address.country)
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "billing.address.country",
            })?;
        let (name, family_name) = match billing_address {
            Some(address) => (address.first_name, address.last_name),
            None => (None, None),
        };

        let lang = item
            .router_data
            .request
            .browser_info
            .as_ref()
            .and_then(|broswer_data| {
                broswer_data.language.as_ref().map(|language| {
                    language
                        .split_once('-')
                        .map_or(language.to_uppercase(), |(lang, _)| lang.to_uppercase())
                })
            })
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "browser_info.language",
            })?;

        let router_url = item.router_data.request.router_return_url.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "router_return_url",
            },
        )?;
        let webhook_url = item.router_data.request.webhook_url.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "webhook_url",
            },
        )?;

        let cust_tag = item
            .router_data
            .customer_id
            .clone()
            .map(|customer_id| {
                let cust_id = customer_id.replace("_", "").replace("-", "");
                let id_len = cust_id.len();
                if id_len > 10 {
                    (&cust_id[id_len - 10 .. id_len]).to_string()
                } else {
                    cust_id
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
            verno: ZSL_VERSION.to_owned(),
            service_code: ServiceCode::MPG,
            country: billing_country,
            payment_method,
            name,
            family_name,
            tel_phone,
            email,
            cust_tag,
            opt_1: None,
            opt_2: None,
            opt_3: None,
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
                    .change_context(errors::ConnectorError::RequestEncodingFailed.into())?;

                let redirect_url = String::from_utf8(decoded_redirect_url_bytes)
                    .change_context(errors::ConnectorError::RequestEncodingFailed.into())?;

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
                    }),
                    ..item.data
                })
            } else {
                // When the signature check fails
                Ok(Self {
                    status: enums::AttemptStatus::Failure,
                    response: Err(types::ErrorResponse {
                        code: consts::NO_ERROR_CODE.to_string(),
                        message: "Invalid Signature".to_string(),
                        reason: Some("Invalid Signature".to_string()),
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
            "0" => api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess,
            _ => api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure,
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
            if item.response.process_type == ProcessType::PaymentResponse
                && item.response.process_code == ProcessCode::CallBack
            {
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
                    }),
                    ..item.data
                })
            } else {
                let error_reason =
                    ZslResponseStatus::try_from(item.response.status.clone())?.to_string();
                Ok(Self {
                    status: enums::AttemptStatus::Unresolved, // What happens when process_type and process_code is not as expected ?
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
            "0" => Ok(ZslResponseStatus::Normal),
            "1000" => Ok(ZslResponseStatus::InternalError),
            "1001" => Ok(ZslResponseStatus::BreakDownMessageError),
            "1002" => Ok(ZslResponseStatus::FormatError),
            "1004" => Ok(ZslResponseStatus::InvalidTransaction),
            "1005" => Ok(ZslResponseStatus::TransactionCountryNotFound),
            "1006" => Ok(ZslResponseStatus::MerchantIdNotFound),
            "1007" => Ok(ZslResponseStatus::AccountDisabled),
            "1008" => Ok(ZslResponseStatus::DuplicateMerchantReference),
            "1009" => Ok(ZslResponseStatus::InvalidPayAmount),
            "1010" => Ok(ZslResponseStatus::PayAmountNotFound),
            "1011" => Ok(ZslResponseStatus::InvalidCurrencyCode),
            "1012" => Ok(ZslResponseStatus::CurrencyCodeNotFound),
            "1013" => Ok(ZslResponseStatus::ReferenceNotFound),
            "1014" => Ok(ZslResponseStatus::TransmissionTimeNotFound),
            "1015" => Ok(ZslResponseStatus::PayMethodNotFound),
            "1016" => Ok(ZslResponseStatus::BankCodeNotFound),
            "1017" => Ok(ZslResponseStatus::InvalidShowPayPage),
            "1018" => Ok(ZslResponseStatus::ShowPayPageNotFound),
            "1019" => Ok(ZslResponseStatus::SuccessUrlNotFound),
            "1020" => Ok(ZslResponseStatus::SuccessCallbackUrlNotFound),
            "1021" => Ok(ZslResponseStatus::FailUrlNotFound),
            "1022" => Ok(ZslResponseStatus::FailCallbackUrlNotFound),
            "1023" => Ok(ZslResponseStatus::MacNotFound),
            "1025" => Ok(ZslResponseStatus::OriginalTransactionNotFound),
            "1026" => Ok(ZslResponseStatus::DeblockDataError),
            "1028" => Ok(ZslResponseStatus::PspAckNotYetReturn),
            "1029" => Ok(ZslResponseStatus::BankBranchNameNotFound),
            "1030" => Ok(ZslResponseStatus::BankAccountIDNotFound),
            "1031" => Ok(ZslResponseStatus::BankAccountNameNotFound),
            "1032" => Ok(ZslResponseStatus::IdentityIDNotFound),
            "1033" => Ok(ZslResponseStatus::ErrorConnectingToPsp),
            "1034" => Ok(ZslResponseStatus::CountryPspNotAvailable),
            "1035" => Ok(ZslResponseStatus::UnsupportedPayAmount),
            "1036" => Ok(ZslResponseStatus::RecordMismatch),
            "1037" => Ok(ZslResponseStatus::NoRecord),
            "1038" => Ok(ZslResponseStatus::PspError),
            "1039" => Ok(ZslResponseStatus::UnsupportedEncryptionType),
            "1040" => Ok(ZslResponseStatus::ExceedTransactionLimitCount),
            "1041" => Ok(ZslResponseStatus::ExceedTransactionLimitAmount),
            "1042" => Ok(ZslResponseStatus::ExceedTransactionAccountLimitCount),
            "1043" => Ok(ZslResponseStatus::ExceedTransactionAccountLimitAmount),
            "1044" => Ok(ZslResponseStatus::ExchangeRateError),
            "1045" => Ok(ZslResponseStatus::InvalidEncoding),
            "1046" => Ok(ZslResponseStatus::CustomerNameNotFound),
            "1047" => Ok(ZslResponseStatus::CustomerFamilyNameNotFound),
            "1048" => Ok(ZslResponseStatus::CustomerTelPhoneNotFound),
            "1049" => Ok(ZslResponseStatus::InsufficientFund),
            "1050" => Ok(ZslResponseStatus::ServiceCodeIsMissing),
            "1051" => Ok(ZslResponseStatus::CurrencyIdNotMatch),
            "1052" => Ok(ZslResponseStatus::NoPendingRecord),
            "1053" => Ok(ZslResponseStatus::NoLoadBalancerRuleDefineForTransaction),
            "1054" => Ok(ZslResponseStatus::NoPaymentProviderAvailable),
            "1055" => Ok(ZslResponseStatus::UnsupportedPayMethod),
            "1056" => Ok(ZslResponseStatus::PendingTransaction),
            "1057" => Ok(ZslResponseStatus::OtherError1059),
            "1058" => Ok(ZslResponseStatus::OtherError1058),
            "1059" => Ok(ZslResponseStatus::OtherError1059),
            "1084" => Ok(ZslResponseStatus::InvalidRequestId),
            "5043" => Ok(ZslResponseStatus::BeneficiaryBankAccountIsNotAvailable),
            "5053" => Ok(ZslResponseStatus::BaidNotFound),
            "5057" => Ok(ZslResponseStatus::InvalidBaid),
            "5059" => Ok(ZslResponseStatus::InvalidBaidStatus),
            "5107" => Ok(ZslResponseStatus::AutoUploadBankDisabled),
            "5108" => Ok(ZslResponseStatus::InvalidNature),
            "5109" => Ok(ZslResponseStatus::SmsCreateDateNotFound),
            "5110" => Ok(ZslResponseStatus::InvalidSmsCreateDate),
            "5111" => Ok(ZslResponseStatus::RecordNotFound),
            "5112" => Ok(ZslResponseStatus::InsufficientBaidAvailableBalance),
            "5113" => Ok(ZslResponseStatus::ExceedTxnAmountLimit),
            "5114" => Ok(ZslResponseStatus::BaidBalanceNotFound),
            "5115" => Ok(ZslResponseStatus::AutoUploadIndicatorNotFound),
            "5116" => Ok(ZslResponseStatus::InvalidBankAcctStatus),
            "5117" => Ok(ZslResponseStatus::InvalidAutoUploadIndicator),
            "5118" => Ok(ZslResponseStatus::InvalidPidStatus),
            "5119" => Ok(ZslResponseStatus::InvalidProviderStatus),
            "5120" => Ok(ZslResponseStatus::InvalidBankAccountSystemSwitchEnabled),
            "5121" => Ok(ZslResponseStatus::AutoUploadProviderDisabled),
            "5122" => Ok(ZslResponseStatus::AutoUploadBankNotFound),
            "5123" => Ok(ZslResponseStatus::AutoUploadBankAcctNotFound),
            "5124" => Ok(ZslResponseStatus::AutoUploadProviderNotFound),
            "5125" => Ok(ZslResponseStatus::UnsupportedBankCode),
            "5126" => Ok(ZslResponseStatus::BalanceOverrideIndicatorNotFound),
            "5127" => Ok(ZslResponseStatus::InvalidBalanceOverrideIndicator),
            "10000" => Ok(ZslResponseStatus::VernoInvalid),
            "10001" => Ok(ZslResponseStatus::ServiceCodeInvalid),
            "10002" => Ok(ZslResponseStatus::PspResponseSignatureIsNotValid),
            "10003" => Ok(ZslResponseStatus::ProcessTypeNotFound),
            "10004" => Ok(ZslResponseStatus::ProcessCodeNotFound),
            "10005" => Ok(ZslResponseStatus::EnctypeNotFound),
            "10006" => Ok(ZslResponseStatus::VernoNotFound),
            "10007" => Ok(ZslResponseStatus::DepositBankNotFound),
            "10008" => Ok(ZslResponseStatus::DepositFlowNotFound),
            "10009" => Ok(ZslResponseStatus::CustDepositDateNotFound),
            "10010" => Ok(ZslResponseStatus::CustTagNotFound),
            "10011" => Ok(ZslResponseStatus::CountryValueInvalid),
            "10012" => Ok(ZslResponseStatus::CurrencyCodeValueInvalid),
            "10013" => Ok(ZslResponseStatus::MerTxnDateInvalid),
            "10014" => Ok(ZslResponseStatus::CustDepositDateInvalid),
            "10015" => Ok(ZslResponseStatus::TxnAmtInvalid),
            "10016" => Ok(ZslResponseStatus::SuccessCallbackUrlInvalid),
            "10017" => Ok(ZslResponseStatus::DepositFlowInvalid),
            "10018" => Ok(ZslResponseStatus::ProcessTypeInvalid),
            "10019" => Ok(ZslResponseStatus::ProcessCodeInvalid),
            "10020" => Ok(ZslResponseStatus::UnsupportedMerRefLength),
            "10021" => Ok(ZslResponseStatus::DepositBankLengthOverLimit),
            "10022" => Ok(ZslResponseStatus::CustTagLengthOverLimit),
            "10023" => Ok(ZslResponseStatus::SignatureLengthOverLimit),
            "10024" => Ok(ZslResponseStatus::RequestContainInvalidTag),
            "10025" => Ok(ZslResponseStatus::RequestSignatureNotMatch),
            "10026" => Ok(ZslResponseStatus::InvalidCustomer),
            "10027" => Ok(ZslResponseStatus::SchemeNotFound),
            "10028" => Ok(ZslResponseStatus::PspResponseFieldsMissing),
            "10029" => Ok(ZslResponseStatus::PspResponseMerRefNotMatchWithRequestMerRef),
            "10030" => Ok(ZslResponseStatus::PspResponseMerIdNotMatchWithRequestMerId),
            "10031" => Ok(ZslResponseStatus::UpdateDepositFailAfterResponse),
            "10032" => {
                Ok(ZslResponseStatus::UpdateUsedLimitTransactionCountFailAfterSuccessResponse)
            }
            "10033" => Ok(ZslResponseStatus::UpdateCustomerLastDepositRecordAfterSuccessResponse),
            "10034" => Ok(ZslResponseStatus::CreateDepositFail),
            "10035" => Ok(ZslResponseStatus::CreateDepositMsgFail),
            "10036" => Ok(ZslResponseStatus::UpdateStatusSubStatusFail),
            "10037" => Ok(ZslResponseStatus::AddDepositRecordToSchemeAccount),
            "10038" => Ok(ZslResponseStatus::EmptyResponse),
            "10039" => Ok(ZslResponseStatus::AubConfirmErrorFromPh),
            "10040" => Ok(ZslResponseStatus::ProviderEmailAddressNotFound),
            "10041" => Ok(ZslResponseStatus::AubConnectionTimeout),
            "10042" => Ok(ZslResponseStatus::AubConnectionIssue),
            "10043" => Ok(ZslResponseStatus::AubMsgTypeMissing),
            "10044" => Ok(ZslResponseStatus::AubMsgCodeMissing),
            "10045" => Ok(ZslResponseStatus::AubVersionMissing),
            "10046" => Ok(ZslResponseStatus::AubEncTypeMissing),
            "10047" => Ok(ZslResponseStatus::AubSignMissing),
            "10048" => Ok(ZslResponseStatus::AubInfoMissing),
            "10049" => Ok(ZslResponseStatus::AubErrorCodeMissing),
            "10050" => Ok(ZslResponseStatus::AubMsgTypeInvalid),
            "10051" => Ok(ZslResponseStatus::AubMsgCodeInvalid),
            "10052" => Ok(ZslResponseStatus::AubBaidMissing),
            "10053" => Ok(ZslResponseStatus::AubResponseSignNotMatch),
            "10054" => Ok(ZslResponseStatus::SmsConnectionTimeout),
            "10055" => Ok(ZslResponseStatus::SmsConnectionIssue),
            "10056" => Ok(ZslResponseStatus::SmsConfirmErrorFromPh),
            "10057" => Ok(ZslResponseStatus::SmsMsgTypeMissing),
            "10058" => Ok(ZslResponseStatus::SmsMsgCodeMissing),
            "10059" => Ok(ZslResponseStatus::SmsVersionMissing),
            "10060" => Ok(ZslResponseStatus::SmsEncTypeMissing),
            "10061" => Ok(ZslResponseStatus::SmsSignMissing),
            "10062" => Ok(ZslResponseStatus::SmsInfoMissing),
            "10063" => Ok(ZslResponseStatus::SmsErrorCodeMissing),
            "10064" => Ok(ZslResponseStatus::SmsMsgTypeInvalid),
            "10065" => Ok(ZslResponseStatus::SmsMsgCodeInvalid),
            "10066" => Ok(ZslResponseStatus::SmsResponseSignNotMatch),
            "10067" => Ok(ZslResponseStatus::SmsRequestReachMaximumLimit),
            "10068" => Ok(ZslResponseStatus::SyncConnectionTimeout),
            "10069" => Ok(ZslResponseStatus::SyncConnectionIssue),
            "10070" => Ok(ZslResponseStatus::SyncConfirmErrorFromPh),
            "10071" => Ok(ZslResponseStatus::SyncMsgTypeMissing),
            "10072" => Ok(ZslResponseStatus::SyncMsgCodeMissing),
            "10073" => Ok(ZslResponseStatus::SyncVersionMissing),
            "10074" => Ok(ZslResponseStatus::SyncEncTypeMissing),
            "10075" => Ok(ZslResponseStatus::SyncSignMissing),
            "10076" => Ok(ZslResponseStatus::SyncInfoMissing),
            "10077" => Ok(ZslResponseStatus::SyncErrorCodeMissing),
            "10078" => Ok(ZslResponseStatus::SyncMsgTypeInvalid),
            "10079" => Ok(ZslResponseStatus::SyncMsgCodeInvalid),
            "10080" => Ok(ZslResponseStatus::SyncResponseSignNotMatch),
            "10081" => Ok(ZslResponseStatus::AccountExpired),
            "10082" => Ok(ZslResponseStatus::ExceedMaxMinAmount),
            "10083" => Ok(ZslResponseStatus::WholeNumberAmountLessThanOne),
            "10084" => Ok(ZslResponseStatus::AddDepositRecordToSchemeChannel),
            "10085" => Ok(ZslResponseStatus::UpdateUtilizedAmountFailAfterSuccessResponse),
            "10086" => Ok(ZslResponseStatus::PidResponseInvalidFormat),
            "10087" => Ok(ZslResponseStatus::PspNameNotFound),
            "10088" => Ok(ZslResponseStatus::LangIsMissing),
            "10089" => Ok(ZslResponseStatus::FailureCallbackUrlInvalid),
            "10090" => Ok(ZslResponseStatus::SuccessRedirectUrlInvalid),
            "10091" => Ok(ZslResponseStatus::FailureRedirectUrlInvalid),
            "10092" => Ok(ZslResponseStatus::LangValueInvalid),
            "10093" => Ok(ZslResponseStatus::OnlineDepositSessionTimeout),
            "10094" => Ok(ZslResponseStatus::AccessPaymentPageRouteFieldMissing),
            "10095" => Ok(ZslResponseStatus::AmountNotMatch),
            "10096" => Ok(ZslResponseStatus::PidCallbackFieldsMissing),
            "10097" => Ok(ZslResponseStatus::TokenNotMatch),
            "10098" => Ok(ZslResponseStatus::OperationDuplicated),
            "10099" => Ok(ZslResponseStatus::PayPageDomainNotAvailable),
            "10100" => Ok(ZslResponseStatus::PayPageConfirmSignatureNotMatch),
            "10101" => Ok(ZslResponseStatus::PaymentPageConfirmationFieldMissing),
            "10102" => Ok(ZslResponseStatus::MultipleCallbackFromPsp),
            "10103" => Ok(ZslResponseStatus::PidNotAvailable),
            "10104" => Ok(ZslResponseStatus::PidDepositUrlNotValidOrEmp),
            "10105" => Ok(ZslResponseStatus::PspSelfRedirectTagNotValid),
            "20000" => Ok(ZslResponseStatus::InternalError20000),
            "20001" => Ok(ZslResponseStatus::DepositTimeout),
            _ => Err(errors::ConnectorError::ResponseHandlingFailed.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, strum::Display)]
pub enum ZslResponseStatus {
    #[default]
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
                .change_context(errors::ConnectorError::RequestEncodingFailed.into())?,
        ),
        EncodingType::Sha1 => {
            hex::encode(digest::digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, message))
        }
    };
    Ok(Secret::new(encoded_data))
}
