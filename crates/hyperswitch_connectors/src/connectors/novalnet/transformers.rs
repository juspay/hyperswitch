use std::collections::HashMap;

use cards::CardNumber;
use common_enums::{enums, enums as api_enums};
use common_utils::{
    ext_traits::OptionExt,
    pii::{Email, IpAddress},
    request::Method,
    types::StringMinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        missing_field_err, BrowserInformationData, PaymentsAuthorizeRequestData,
        PaymentsCancelRequestData, PaymentsCaptureRequestData, PaymentsSyncRequestData,
        RefundsRequestData, RouterData as OtherRouterData,
    },
};

//TODO: Fill the struct with respective fields
pub struct NovalnetRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for NovalnetRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo : use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}
#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum PaymentType {
    Card,
    Applepay,
    Googlepay,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NovalNetPaymentTypes {
    CREDITCARD,
}

#[derive(Default, Debug, Serialize, PartialEq, Clone)]
pub struct NovalnetPaymentsRequestMerchant {
    signature: Secret<String>,
    tariff: Secret<String>,
}

#[derive(Default, Debug, Serialize, PartialEq, Clone)]
pub struct NovalnetPaymentsRequestBilling {
    house_no: Option<Secret<String>>,
    street: Option<Secret<String>>,
    city: Option<String>,
    zip: Option<Secret<String>>,
    country_code: Option<api_enums::CountryAlpha2>,
}

#[derive(Default, Debug, Serialize, PartialEq, Clone)]
pub struct NovalnetPaymentsRequestCustomer {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    email: Option<Email>,
    mobile: Option<Secret<String>>,
    billing: NovalnetPaymentsRequestBilling,
    customer_ip: Secret<String, IpAddress>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize)]

pub struct NovalNetCard {
    card_number: CardNumber,
    card_expiry_month: Secret<String>,
    card_expiry_year: Secret<String>,
    card_cvc: Option<Secret<String>>,
    card_holder: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum NovalNetPaymentData {
    PaymentCard(NovalNetCard),
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct NovalnetCustom {
    lang: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NovalnetPaymentsRequestTransaction {
    test_mode: i8,
    payment_type: NovalNetPaymentTypes,
    amount: Option<StringMinorUnit>,
    currency: Option<String>,
    order_no: Option<String>,
    payment_data: NovalNetPaymentData,
    hook_url: Option<String>,
    return_url: Option<String>,
    error_return_url: Option<String>,
    enforce_3d: Option<i8>,
}

#[derive(Debug, Serialize, Clone)]
pub struct NovalnetPaymentsRequest {
    merchant: NovalnetPaymentsRequestMerchant,
    customer: NovalnetPaymentsRequestCustomer,
    transaction: NovalnetPaymentsRequestTransaction,
    custom: NovalnetCustom,
}

type Error = error_stack::Report<errors::ConnectorError>;
fn result_to_option(result: Result<String, Error>) -> Option<String> {
    result.ok()
}
fn result_to_option_secret_string(result: Result<Secret<String>, Error>) -> Option<Secret<String>> {
    result.ok()
}

impl TryFrom<&NovalnetRouterData<&PaymentsAuthorizeRouterData>> for NovalnetPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NovalnetRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let auth = NovalnetAuthType::try_from(&item.router_data.connector_auth_type)?;

                let merchant = NovalnetPaymentsRequestMerchant {
                    signature: auth.product_activation_key,
                    tariff: auth.tariff_id,
                };

                let novalnet_card = NovalNetPaymentData::PaymentCard(NovalNetCard {
                    card_number: req_card.card_number,
                    card_expiry_month: req_card.card_exp_month,
                    card_expiry_year: req_card.card_exp_year,
                    card_cvc: Some(req_card.card_cvc),
                    card_holder: item.router_data.get_optional_billing_full_name(),
                });

                let enforce_3d = match item.router_data.auth_type {
                    enums::AuthenticationType::ThreeDs => Some(1),
                    enums::AuthenticationType::NoThreeDs => None,
                };
                let test_mode = match item.router_data.test_mode {
                    Some(true) => 1,
                    Some(false) | None => 0,
                };

                let return_url = result_to_option(item.router_data.request.get_return_url());
                let hook_url = result_to_option(item.router_data.request.get_webhook_url());
                let transaction = NovalnetPaymentsRequestTransaction {
                    test_mode,
                    payment_type: NovalNetPaymentTypes::CREDITCARD,
                    amount: Some(item.amount.clone()),
                    currency: Some(item.router_data.request.currency.to_string()),
                    order_no: Some(item.router_data.connector_request_reference_id.clone()),
                    hook_url,
                    return_url: return_url.clone(),
                    error_return_url: return_url.clone(),
                    payment_data: novalnet_card,
                    enforce_3d,
                };

                let billing = NovalnetPaymentsRequestBilling {
                    house_no: item.router_data.get_optional_billing_line1(),
                    street: item.router_data.get_optional_billing_line2(),
                    city: item.router_data.get_optional_billing_city(),
                    zip: item.router_data.get_optional_billing_zip(),
                    country_code: item.router_data.get_optional_billing_country(),
                };

                let customer_ip = item
                    .router_data
                    .request
                    .get_browser_info()?
                    .get_ip_address()?;

                let customer = NovalnetPaymentsRequestCustomer {
                    first_name: result_to_option_secret_string(
                        item.router_data.get_billing_first_name(),
                    ),
                    last_name: item.router_data.get_optional_billing_last_name(),
                    email: item.router_data.get_optional_billing_email(),
                    mobile: result_to_option_secret_string(
                        item.router_data.get_billing_phone_number(),
                    ),
                    billing: billing,
                    customer_ip: customer_ip,
                };

                let lang = item
                    .router_data
                    .request
                    .get_browser_info()?
                    .get_language()?;

                let custom = NovalnetCustom { lang: lang };

                Ok(NovalnetPaymentsRequest {
                    merchant,
                    transaction,
                    customer,
                    custom,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct NovalnetAuthType {
    pub(super) product_activation_key: Secret<String>,
    pub(super) payment_access_key: Secret<String>,
    pub(super) tariff_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NovalnetAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                product_activation_key: api_key.to_owned(),
                payment_access_key: key1.to_owned(),
                tariff_id: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
#[allow(non_camel_case_types)]
pub enum NovalnetTransactionStatus {
    SUCCESS,
    FAILURE,
    CONFIRMED,
    ON_HOLD,
    PENDING,
    #[default]
    DEACTIVATED,
    PROGRESS,
}

#[derive(Debug, Display, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
#[allow(non_camel_case_types)]
pub enum NovalnetAPIStatus {
    SUCCESS,
    #[default]
    FAILURE,
}

impl From<NovalnetTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: NovalnetTransactionStatus) -> Self {
        match item {
            NovalnetTransactionStatus::SUCCESS | NovalnetTransactionStatus::CONFIRMED => {
                Self::Charged
            }
            NovalnetTransactionStatus::ON_HOLD => Self::Authorized,
            NovalnetTransactionStatus::PENDING => Self::Pending,
            NovalnetTransactionStatus::PROGRESS => Self::AuthenticationPending,
            NovalnetTransactionStatus::DEACTIVATED => Self::Voided,
            NovalnetTransactionStatus::FAILURE => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResultData {
    redirect_url: Option<url::Url>,
    status: NovalnetAPIStatus,
    status_code: i32,
    status_text: String,
    additional_message: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    payment_type: String,
    status_code: i32,
    txn_secret: Option<String>,
    tid: Option<u64>,
    test_mode: Option<i8>,
    status: Option<NovalnetTransactionStatus>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovalnetPaymentsResponse {
    result: ResultData,
    transaction: Option<TransactionData>,
}

pub fn get_error_response(result: ResultData, statusCode: u16) -> ErrorResponse {
    let error_code = result.status;
    let error_reason = result.status_text.clone();

    ErrorResponse {
        code: error_code.to_string(),
        message: error_reason.clone(),
        reason: Some(error_reason),
        status_code: statusCode,
        attempt_status: None,
        connector_transaction_id: None,
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, NovalnetPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NovalnetPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.result.status {
            NovalnetAPIStatus::SUCCESS => {
                let redirection_data: Option<RedirectForm> =
                    item.response
                        .result
                        .redirect_url
                        .map(|x| RedirectForm::Form {
                            endpoint: x.to_string(),
                            method: Method::Get,
                            form_fields: HashMap::new(),
                        });

                let transaction_id = item
                    .response
                    .transaction
                    .as_ref()
                    .and_then(|transaction| transaction.tid.clone());

                let transaction_status = item
                    .response
                    .transaction
                    .as_ref()
                    .and_then(|transaction| {
                        if transaction.status_code == 100 {
                            // TODO: verify status_code
                            transaction.status.clone()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(NovalnetTransactionStatus::PROGRESS);

                Ok(Self {
                    status: common_enums::AttemptStatus::from(transaction_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: match transaction_id.clone() {
                            Some(id) => ResponseId::ConnectorTransactionId(id.to_string()),
                            None => ResponseId::NoResponseId,
                        },
                        redirection_data,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id.map(|id| id.to_string()),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::FAILURE => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseCustomer {
    pub billing: NovalnetResponseBilling,
    pub customer_ip: String,
    pub email: String,
    pub first_name: String,
    pub gender: String,
    pub last_name: String,
    pub mobile: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseBilling {
    pub city: String,
    pub country_code: String,
    pub house_no: String,
    pub street: String,
    pub zip: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseMerchant {
    pub project: u32,
    pub vendor: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseTransactionData {
    pub amount: u32,
    pub currency: String,
    pub date: Option<String>,
    pub order_no: String,
    pub payment_data: NovalnetResponsePaymentData,
    pub payment_type: String,
    pub status: NovalnetTransactionStatus,
    pub status_code: u16,
    pub test_mode: u8,
    pub tid: u64,
    pub txn_secret: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum NovalnetResponsePaymentData {
    PaymentCard(NovalnetResponseCard),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseCard {
    pub card_brand: String,
    pub card_expiry_month: u8,
    pub card_expiry_year: u16,
    pub card_holder: String,
    pub card_number: String,
    pub cc_3d: u8,
    pub last_four: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetPSyncResponse {
    pub customer: Option<NovalnetResponseCustomer>,
    pub merchant: Option<NovalnetResponseMerchant>,
    pub result: ResultData,
    pub transaction: Option<NovalnetResponseTransactionData>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum CaptureType {
    #[default]
    PARTIAL,
    FINAL,
}

#[derive(Default, Debug, Serialize)]
pub struct Capture {
    _type: CaptureType,
    reference: String,
}
#[derive(Default, Debug, Serialize)]
pub struct NovalnetTransaction {
    tid: String,
    amount: Option<StringMinorUnit>,
    capture: Option<Capture>,
}

#[derive(Default, Debug, Serialize)]
pub struct NovalnetCaptureRequest {
    pub transaction: NovalnetTransaction,
    pub custom: NovalnetCustom,
}

impl TryFrom<&NovalnetRouterData<&PaymentsCaptureRouterData>> for NovalnetCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NovalnetRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let capture_type = if item.router_data.request.is_multiple_capture() {
            CaptureType::PARTIAL
        } else {
            CaptureType::FINAL
        };

        let reference = item
            .router_data
            .request
            .multiple_capture_data
            .as_ref()
            .map(|multiple_capture_data| multiple_capture_data.capture_reference.clone());

        let capture: Option<Capture> = match reference {
            Some(reference) => Some(Capture {
                _type: capture_type,
                reference,
            }),
            None => None,
        };

        let transaction = NovalnetTransaction {
            tid: item.router_data.request.connector_transaction_id.clone(),
            capture,
            amount: Some(item.amount.to_owned()),
        };

        let custom = NovalnetCustom {
            lang: item
                .router_data
                .request
                .get_browser_info()?
                .get_language()?,
        };
        Ok(NovalnetCaptureRequest {
            transaction,
            custom,
        })
    }
}

// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct NovalnetRefundTransaction {
    tid: String,
    amount: Option<StringMinorUnit>,
}

#[derive(Default, Debug, Serialize)]
pub struct NovalnetRefundRequest {
    pub transaction: NovalnetRefundTransaction,
    pub custom: NovalnetCustom,
}

impl<F> TryFrom<&NovalnetRouterData<&RefundsRouterData<F>>> for NovalnetRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &NovalnetRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let transaction = NovalnetRefundTransaction {
            tid: item.router_data.request.connector_transaction_id.clone(),
            amount: Some(item.amount.to_owned()),
        };

        let custom = NovalnetCustom {
            lang: item
                .router_data
                .request
                .get_browser_info()?
                .get_language()?,
        };
        Ok(NovalnetRefundRequest {
            transaction,
            custom,
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[allow(non_camel_case_types)]
pub enum NovalnetRefundStatus {
    SUCCESS,
    FAILURE,
    CONFIRMED,
    ON_HOLD,
    PENDING,
    #[default]
    DEACTIVATED,
}

impl From<NovalnetTransactionStatus> for enums::RefundStatus {
    fn from(item: NovalnetTransactionStatus) -> Self {
        match item {
            NovalnetTransactionStatus::SUCCESS | NovalnetTransactionStatus::CONFIRMED => {
                Self::Success
            }
            NovalnetTransactionStatus::PENDING => Self::Pending,
            _ => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetRefundSyncResponse {
    result: ResultData,
    transaction: Option<NovalnetResponseTransactionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetRefundsTransactionData {
    amount: u32,
    date: Option<String>,
    currency: String,
    order_no: String,
    payment_type: String,
    refund: RefundData,
    refunded_amount: u32,
    status: NovalnetTransactionStatus,
    status_code: u16,
    test_mode: u8,
    tid: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundData {
    amount: u32,
    currency: String,
    payment_type: String,
    tid: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetRefundResponse {
    pub customer: Option<NovalnetResponseCustomer>,
    pub merchant: Option<NovalnetResponseMerchant>,
    pub result: ResultData,
    pub transaction: Option<NovalnetRefundsTransactionData>,
}

impl TryFrom<RefundsResponseRouterData<Execute, NovalnetRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, NovalnetRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.result.status {
            NovalnetAPIStatus::SUCCESS => {
                let get_refund_id = match item.response.transaction.clone() {
                    Some(transaction) => Some(transaction.tid),
                    None => None,
                }
                .ok_or_else(missing_field_err("transaction id"))?;

                let transaction_status = match item.response.transaction.clone() {
                    Some(transaction) => Some(transaction.status),
                    None => None,
                }
                .ok_or_else(missing_field_err("transaction status"))?;

                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: get_refund_id.expect("REASON").to_string(),
                        refund_status: enums::RefundStatus::from(transaction_status),
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::FAILURE => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct NovolnetRedirectionResponse {
    status: NovalnetTransactionStatus,
    tid: String,
}

impl TryFrom<&PaymentsSyncRouterData> for NovalnetSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let transaction = if item
            .request
            .encoded_data
            .clone()
            .get_required_value("encoded_data")
            .is_ok()
        {
            let encoded_data = item
                .request
                .encoded_data
                .clone()
                .get_required_value("encoded_data")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            let novalnet_redirection_response =
                serde_urlencoded::from_str::<NovolnetRedirectionResponse>(encoded_data.as_str())
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            NovalnetSyncTransaction {
                tid: novalnet_redirection_response.tid,
            }
        } else {
            NovalnetSyncTransaction {
                tid: item
                    .request
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
            }
        };

        let custom = NovalnetCustom {
            lang: "EN".to_string(),
        };
        Ok(NovalnetSyncRequest {
            transaction,
            custom,
        })
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, NovalnetPSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NovalnetPSyncResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.result.status {
            NovalnetAPIStatus::SUCCESS => {
                let transaction_id = match item.response.transaction.clone() {
                    Some(transaction) => Some(transaction.tid),
                    None => None,
                };

                let transaction_status = match item.response.transaction {
                    Some(transaction) => Some(transaction.status),
                    None => None,
                }
                .unwrap_or(NovalnetTransactionStatus::PROGRESS);

                Ok(Self {
                    status: common_enums::AttemptStatus::from(transaction_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: match transaction_id.clone() {
                            Some(id) => ResponseId::ConnectorTransactionId(id.to_string()),
                            None => ResponseId::NoResponseId,
                        },
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id
                            .map(|id| id.to_string().clone()),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::FAILURE => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureTransactionData {
    amount: Option<u32>,
    capture: CaptureData,
    currency: Option<String>,
    order_no: Option<String>,
    payment_type: Option<String>,
    status: Option<NovalnetTransactionStatus>,
    status_code: Option<u16>,
    test_mode: Option<u8>,
    tid: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureData {
    amount: Option<u32>,
    payment_type: Option<String>,
    status: Option<String>,
    status_code: u16,
    tid: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetCaptureResponse {
    pub result: ResultData,
    pub transaction: Option<CaptureTransactionData>,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, NovalnetCaptureResponse, PaymentsCaptureData, PaymentsResponseData>,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NovalnetCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.result.status {
            NovalnetAPIStatus::SUCCESS => {
                let transaction_id = match item.response.transaction.clone() {
                    Some(transaction) => Some(transaction.tid),
                    None => None,
                };

                let transaction_status = match item.response.transaction {
                    Some(transaction) => transaction.status,
                    None => None,
                }
                .unwrap_or(NovalnetTransactionStatus::PROGRESS);

                Ok(Self {
                    status: common_enums::AttemptStatus::from(transaction_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: match transaction_id.clone() {
                            Some(id) => {
                                ResponseId::ConnectorTransactionId(id.expect("REASON").to_string())
                            }
                            None => ResponseId::NoResponseId,
                        },
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id
                            .map(|id| id.expect("REASON").to_string().clone()),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::FAILURE => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct NovalnetSyncTransaction {
    tid: String,
}

#[derive(Default, Debug, Serialize)]
pub struct NovalnetSyncRequest {
    pub transaction: NovalnetSyncTransaction,
    pub custom: NovalnetCustom,
}

impl TryFrom<&RefundSyncRouterData> for NovalnetSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let transaction = NovalnetSyncTransaction {
            tid: item.request.connector_transaction_id.clone(),
        };

        let custom = NovalnetCustom {
            lang: item.request.get_browser_info()?.get_language()?,
        };
        Ok(NovalnetSyncRequest {
            transaction,
            custom,
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, NovalnetRefundSyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, NovalnetRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.result.status {
            NovalnetAPIStatus::SUCCESS => {
                let refund_id = match item.response.transaction.clone() {
                    Some(transaction) => Some(transaction.tid),
                    None => None,
                }
                .ok_or_else(missing_field_err("transaction id"))?;

                let transaction_status = match item.response.transaction.clone() {
                    Some(transaction) => Some(transaction.status),
                    None => None,
                }
                .ok_or_else(missing_field_err("transaction status"))?;

                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: refund_id.to_string(),
                        refund_status: enums::RefundStatus::from(transaction_status),
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::FAILURE => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct NovalnetCancelTransaction {
    tid: String,
}

#[derive(Default, Debug, Serialize)]
pub struct NovalnetCancelRequest {
    pub transaction: NovalnetCancelTransaction,
    pub custom: NovalnetCustom,
}

impl TryFrom<&PaymentsCancelRouterData> for NovalnetCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let transaction = NovalnetCancelTransaction {
            tid: item.request.connector_transaction_id.clone(),
        };

        let custom = NovalnetCustom {
            lang: item.request.get_browser_info()?.get_language()?,
        };
        Ok(NovalnetCancelRequest {
            transaction,
            custom,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovalnetCancelResponse {
    result: ResultData,
    transaction: Option<TransactionData>,
}

impl<F>
    TryFrom<ResponseRouterData<F, NovalnetCancelResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            NovalnetCancelResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.result.status {
            NovalnetAPIStatus::SUCCESS => {
                let transaction_id = match item.response.transaction.clone() {
                    Some(transaction) => match transaction.tid.clone() {
                        Some(tid) => Some(tid),
                        None => None,
                    },
                    None => None,
                };//

                let transaction_status = match item.response.transaction {
                    Some(transaction) => transaction.status,
                    None => None,
                }//
                .unwrap_or(NovalnetTransactionStatus::PROGRESS);

                Ok(Self {
                    status: if transaction_status == NovalnetTransactionStatus::DEACTIVATED {
                        enums::AttemptStatus::Voided
                    } else {
                        enums::AttemptStatus::VoidFailed
                    },
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: match transaction_id.clone() {
                            Some(id) => ResponseId::ConnectorTransactionId(id.to_string()),
                            None => ResponseId::NoResponseId,
                        },
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id
                            .map(|id| id.to_string().clone()),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::FAILURE => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::Failure,
                    ..item.data
                })
            }
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NovalnetErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
