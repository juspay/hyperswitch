use std::collections::HashMap;

use api_models::webhooks::IncomingWebhookEvent;
use cards::CardNumber;
use common_enums::{enums, enums as api_enums};
use common_utils::{
    consts,
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
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, BrowserInformationData, PaymentsAuthorizeRequestData, PaymentsCancelRequestData,
        PaymentsCaptureRequestData, PaymentsSyncRequestData, RefundsRequestData,
        RouterData as OtherRouterData,
    },
};

pub struct NovalnetRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for NovalnetRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Copy, Serialize, Deserialize, Clone)]
pub enum NovalNetPaymentTypes {
    CREDITCARD,
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct NovalnetPaymentsRequestMerchant {
    signature: Secret<String>,
    tariff: Secret<String>,
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct NovalnetPaymentsRequestBilling {
    house_no: Secret<String>,
    street: Secret<String>,
    city: Secret<String>,
    zip: Secret<String>,
    country_code: api_enums::CountryAlpha2,
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct NovalnetPaymentsRequestCustomer {
    first_name: Secret<String>,
    last_name: Secret<String>,
    email: Email,
    mobile: Option<Secret<String>>,
    billing: NovalnetPaymentsRequestBilling,
    customer_ip: Secret<String, IpAddress>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize)]

pub struct NovalnetCard {
    card_number: CardNumber,
    card_expiry_month: Secret<String>,
    card_expiry_year: Secret<String>,
    card_cvc: Secret<String>,
    card_holder: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NovalnetMandate {
    token: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum NovalNetPaymentData {
    PaymentCard(NovalnetCard),
    MandatePayment(NovalnetMandate),
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct NovalnetCustom {
    lang: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NovalnetPaymentsRequestTransaction {
    test_mode: i8,
    payment_type: NovalNetPaymentTypes,
    amount: StringMinorUnit,
    currency: common_enums::Currency,
    order_no: String,
    payment_data: NovalNetPaymentData,
    hook_url: Option<String>,
    return_url: Option<String>,
    error_return_url: Option<String>,
    enforce_3d: Option<i8>, //NOTE: Needed for CREDITCARD, GOOGLEPAY
    create_token: Option<i8>,
}

#[derive(Debug, Serialize, Clone)]
pub struct NovalnetPaymentsRequest {
    merchant: NovalnetPaymentsRequestMerchant,
    customer: NovalnetPaymentsRequestCustomer,
    transaction: NovalnetPaymentsRequestTransaction,
    custom: NovalnetCustom,
}

impl TryFrom<&NovalnetRouterData<&PaymentsAuthorizeRouterData>> for NovalnetPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NovalnetRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = NovalnetAuthType::try_from(&item.router_data.connector_auth_type)?;

        let merchant = NovalnetPaymentsRequestMerchant {
            signature: auth.product_activation_key,
            tariff: auth.tariff_id,
        };

        let enforce_3d = match item.router_data.auth_type {
            enums::AuthenticationType::ThreeDs => Some(1),
            enums::AuthenticationType::NoThreeDs => None,
        };
        let test_mode = match item.router_data.test_mode {
            Some(true) => 1,
            Some(false) | None => 0,
        };

        let billing = NovalnetPaymentsRequestBilling {
            house_no: item.router_data.get_billing_line1()?,
            street: item.router_data.get_billing_line2()?,
            city: Secret::new(item.router_data.get_billing_city()?),
            zip: item.router_data.get_billing_zip()?,
            country_code: item.router_data.get_billing_country()?,
        };

        let customer_ip = item
            .router_data
            .request
            .get_browser_info()?
            .get_ip_address()?;

        let customer = NovalnetPaymentsRequestCustomer {
            first_name: item.router_data.get_billing_first_name()?,
            last_name: item.router_data.get_billing_last_name()?,
            email: item.router_data.get_billing_email()?,
            mobile: item.router_data.get_optional_billing_phone_number(),
            billing,
            customer_ip,
        };

        let lang = item
            .router_data
            .request
            .get_optional_language_from_browser_info()
            .unwrap_or(consts::DEFAULT_LOCALE.to_string().to_string());
        let custom = NovalnetCustom { lang };
        let hook_url = item.router_data.request.get_webhook_url()?;

        match item
            .router_data
            .request
            .mandate_id
            .clone()
            .and_then(|mandate_id| mandate_id.mandate_reference_id)
        {
            None => match item.router_data.request.payment_method_data {
                PaymentMethodData::Card(ref req_card) => {
                    let novalnet_card = NovalNetPaymentData::PaymentCard(NovalnetCard {
                        card_number: req_card.card_number.clone(),
                        card_expiry_month: req_card.card_exp_month.clone(),
                        card_expiry_year: req_card.card_exp_year.clone(),
                        card_cvc: req_card.card_cvc.clone(),
                        card_holder: item.router_data.get_billing_full_name()?,
                    });
                    let create_token = if item.router_data.request.is_mandate_payment() {
                        Some(1)
                    } else {
                        None
                    };
                    let return_url = item.router_data.request.get_return_url()?;

                    let transaction = NovalnetPaymentsRequestTransaction {
                        test_mode,
                        payment_type: NovalNetPaymentTypes::CREDITCARD,
                        amount: item.amount.clone(),
                        currency: item.router_data.request.currency,
                        order_no: item.router_data.connector_request_reference_id.clone(),
                        hook_url: Some(hook_url),
                        return_url: Some(return_url.clone()),
                        error_return_url: Some(return_url.clone()),
                        payment_data: novalnet_card,
                        enforce_3d,
                        create_token,
                    };

                    Ok(Self {
                        merchant,
                        transaction,
                        customer,
                        custom,
                    })
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("novalnet"),
                )
                .into()),
            },
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(mandate_data)) => {
                let connector_mandate_id = mandate_data.connector_mandate_id.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "connector_mandate_id",
                    },
                )?;

                let novalnet_mandate_data = NovalNetPaymentData::MandatePayment(NovalnetMandate {
                    token: Secret::new(connector_mandate_id),
                });

                let transaction = NovalnetPaymentsRequestTransaction {
                    test_mode,
                    payment_type: NovalNetPaymentTypes::CREDITCARD,
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                    order_no: item.router_data.connector_request_reference_id.clone(),
                    hook_url: Some(hook_url),
                    return_url: None,
                    error_return_url: None,
                    payment_data: novalnet_mandate_data,
                    enforce_3d,
                    create_token: None,
                };

                Ok(Self {
                    merchant,
                    transaction,
                    customer,
                    custom,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("novalnet"),
            )
            .into()),
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
#[derive(Debug, Display, Copy, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NovalnetTransactionStatus {
    Success,
    Failure,
    Confirmed,
    OnHold,
    Pending,
    Deactivated,
    Progress,
}

#[derive(Debug, Copy, Display, Clone, Serialize, Deserialize, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum NovalnetAPIStatus {
    Success,
    Failure,
}

impl From<NovalnetTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: NovalnetTransactionStatus) -> Self {
        match item {
            NovalnetTransactionStatus::Success | NovalnetTransactionStatus::Confirmed => {
                Self::Charged
            }
            NovalnetTransactionStatus::OnHold => Self::Authorized,
            NovalnetTransactionStatus::Pending => Self::Pending,
            NovalnetTransactionStatus::Progress => Self::AuthenticationPending,
            NovalnetTransactionStatus::Deactivated => Self::Voided,
            NovalnetTransactionStatus::Failure => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultData {
    pub redirect_url: Option<Secret<url::Url>>,
    pub status: NovalnetAPIStatus,
    pub status_code: u64,
    pub status_text: String,
    pub additional_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetPaymentsResponseTransactionData {
    pub amount: Option<u64>,
    pub currency: Option<common_enums::Currency>,
    pub date: Option<String>,
    pub order_no: Option<String>,
    pub payment_data: Option<NovalnetResponsePaymentData>,
    pub payment_type: Option<String>,
    pub status_code: Option<u64>,
    pub txn_secret: Option<Secret<String>>,
    pub tid: Option<Secret<i64>>,
    pub test_mode: Option<i8>,
    pub status: Option<NovalnetTransactionStatus>,
    pub authorization: Option<NovalnetAuthorizationResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetPaymentsResponse {
    result: ResultData,
    transaction: Option<NovalnetPaymentsResponseTransactionData>,
}

pub fn get_error_response(result: ResultData, status_code: u16) -> ErrorResponse {
    let error_code = result.status;
    let error_reason = result.status_text.clone();

    ErrorResponse {
        code: error_code.to_string(),
        message: error_reason.clone(),
        reason: Some(error_reason),
        status_code,
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
            NovalnetAPIStatus::Success => {
                let redirection_data: Option<RedirectForm> =
                    item.response
                        .result
                        .redirect_url
                        .map(|url| RedirectForm::Form {
                            endpoint: url.expose().to_string(),
                            method: Method::Get,
                            form_fields: HashMap::new(),
                        });

                let transaction_id = item
                    .response
                    .transaction
                    .clone()
                    .and_then(|data| data.tid.map(|tid| tid.expose().to_string()));
                let transaction_status = item
                    .response
                    .transaction
                    .and_then(|transaction_data| transaction_data.status)
                    .unwrap_or(if redirection_data.is_some() {
                        NovalnetTransactionStatus::Progress
                    } else {
                        NovalnetTransactionStatus::Pending
                    });
                // NOTE: if result.status is success, we should always get a redirection url for 3DS flow
                // since Novalnet does not always send the transaction.status
                // so default value is kept as Progress if flow is 3ds, otherwise default value is kept as Pending

                Ok(Self {
                    status: common_enums::AttemptStatus::from(transaction_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: transaction_id
                            .clone()
                            .map(ResponseId::ConnectorTransactionId)
                            .unwrap_or(ResponseId::NoResponseId),
                        redirection_data,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id.clone(),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::Failure => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseCustomer {
    pub billing: Option<NovalnetResponseBilling>,
    pub customer_ip: Option<Secret<String>>,
    pub email: Option<Email>,
    pub first_name: Option<Secret<String>>,
    pub gender: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub mobile: Option<Secret<String>>,
    pub tel: Option<Secret<String>>,
    pub fax: Option<Secret<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseBilling {
    pub city: Secret<String>,
    pub country_code: Secret<String>,
    pub house_no: Option<Secret<String>>,
    pub street: Secret<String>,
    pub zip: Secret<String>,
    pub state: Option<Secret<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseMerchant {
    pub project: Option<Secret<i64>>,
    pub project_name: Option<Secret<String>>,
    pub project_url: Option<url::Url>,
    pub vendor: Option<Secret<i64>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetAuthorizationResponse {
    expiry_date: Option<String>,
    auto_action: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetSyncResponseTransactionData {
    pub amount: Option<u64>,
    pub currency: Option<common_enums::Currency>,
    pub date: Option<String>,
    pub order_no: Option<String>,
    pub payment_data: Option<NovalnetResponsePaymentData>,
    pub payment_type: String,
    pub status: NovalnetTransactionStatus,
    pub status_code: u64,
    pub test_mode: u8,
    pub tid: Option<Secret<i64>>,
    pub txn_secret: Option<Secret<String>>,
    pub authorization: Option<NovalnetAuthorizationResponse>,
    pub reason: Option<String>,
    pub reason_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum NovalnetResponsePaymentData {
    PaymentCard(NovalnetResponseCard),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NovalnetResponseCard {
    pub card_brand: Option<Secret<String>>,
    pub card_expiry_month: Secret<u8>,
    pub card_expiry_year: Secret<u16>,
    pub card_holder: Secret<String>,
    pub card_number: Secret<String>,
    pub cc_3d: Option<Secret<u8>>,
    pub last_four: Option<Secret<String>>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetPSyncResponse {
    pub customer: Option<NovalnetResponseCustomer>,
    pub merchant: Option<NovalnetResponseMerchant>,
    pub result: ResultData,
    pub transaction: Option<NovalnetSyncResponseTransactionData>,
}

#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
pub enum CaptureType {
    #[default]
    Partial,
    Final,
}

#[derive(Default, Debug, Serialize)]
pub struct Capture {
    #[serde(rename = "type")]
    cap_type: CaptureType,
    reference: String,
}
#[derive(Default, Debug, Serialize)]
pub struct NovalnetTransaction {
    tid: String,
    amount: Option<StringMinorUnit>,
    capture: Capture,
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
        let capture_type = CaptureType::Final;
        let reference = item.router_data.connector_request_reference_id.clone();
        let capture = Capture {
            cap_type: capture_type,
            reference,
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
                .get_optional_language_from_browser_info()
                .unwrap_or(consts::DEFAULT_LOCALE.to_string()),
        };
        Ok(Self {
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
                .get_optional_language_from_browser_info()
                .unwrap_or(consts::DEFAULT_LOCALE.to_string().to_string()),
        };
        Ok(Self {
            transaction,
            custom,
        })
    }
}

impl From<NovalnetTransactionStatus> for enums::RefundStatus {
    fn from(item: NovalnetTransactionStatus) -> Self {
        match item {
            NovalnetTransactionStatus::Success | NovalnetTransactionStatus::Confirmed => {
                Self::Success
            }
            NovalnetTransactionStatus::Pending => Self::Pending,
            NovalnetTransactionStatus::Failure
            | NovalnetTransactionStatus::OnHold
            | NovalnetTransactionStatus::Deactivated
            | NovalnetTransactionStatus::Progress => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetRefundSyncResponse {
    result: ResultData,
    transaction: Option<NovalnetSyncResponseTransactionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetRefundsTransactionData {
    pub amount: Option<u64>,
    pub date: Option<String>,
    pub currency: Option<common_enums::Currency>,
    pub order_no: Option<String>,
    pub payment_type: String,
    pub refund: RefundData,
    pub refunded_amount: u64,
    pub status: NovalnetTransactionStatus,
    pub status_code: u64,
    pub test_mode: u8,
    pub tid: Option<Secret<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundData {
    amount: u64,
    currency: common_enums::Currency,
    payment_type: String,
    tid: Option<Secret<i64>>,
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
            NovalnetAPIStatus::Success => {
                let refund_id = item
                    .response
                    .transaction
                    .clone()
                    .and_then(|data| data.refund.tid.map(|tid| tid.expose().to_string()))
                    .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

                let transaction_status = item
                    .response
                    .transaction
                    .map(|transaction| transaction.status)
                    .unwrap_or(NovalnetTransactionStatus::Pending);

                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: refund_id,
                        refund_status: enums::RefundStatus::from(transaction_status),
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::Failure => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NovolnetRedirectionResponse {
    status: NovalnetTransactionStatus,
    tid: Secret<String>,
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
                tid: novalnet_redirection_response.tid.expose(),
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
            lang: consts::DEFAULT_LOCALE.to_string().to_string(),
        };
        Ok(Self {
            transaction,
            custom,
        })
    }
}

impl NovalnetSyncResponseTransactionData {
    pub fn get_token(transaction_data: Option<&Self>) -> Option<String> {
        if let Some(payment_data) =
            transaction_data.and_then(|transaction_data| transaction_data.payment_data.clone())
        {
            match &payment_data {
                NovalnetResponsePaymentData::PaymentCard(card_data) => card_data.token.clone(),
            }
        } else {
            None
        }
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
            NovalnetAPIStatus::Success => {
                let transaction_id = item
                    .response
                    .transaction
                    .clone()
                    .and_then(|data| data.tid)
                    .map(|tid| tid.expose().to_string());
                let transaction_status = item
                    .response
                    .transaction
                    .clone()
                    .map(|transaction_data| transaction_data.status)
                    .unwrap_or(NovalnetTransactionStatus::Pending);
                let mandate_reference_id = NovalnetSyncResponseTransactionData::get_token(
                    item.response.transaction.as_ref(),
                );

                Ok(Self {
                    status: common_enums::AttemptStatus::from(transaction_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: transaction_id
                            .clone()
                            .map(ResponseId::ConnectorTransactionId)
                            .unwrap_or(ResponseId::NoResponseId),
                        redirection_data: None,
                        mandate_reference: mandate_reference_id.as_ref().map(|id| {
                            MandateReference {
                                connector_mandate_id: Some(id.clone()),
                                payment_method_id: None,
                                mandate_metadata: None,
                            }
                        }),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id.clone(),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::Failure => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetCaptureTransactionData {
    pub amount: Option<u64>,
    pub capture: CaptureData,
    pub currency: Option<common_enums::Currency>,
    pub order_no: Option<String>,
    pub payment_type: String,
    pub status: NovalnetTransactionStatus,
    pub status_code: Option<u64>,
    pub test_mode: Option<u8>,
    pub tid: Secret<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureData {
    amount: Option<u64>,
    payment_type: Option<String>,
    status: Option<String>,
    status_code: u64,
    tid: Option<Secret<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetCaptureResponse {
    pub result: ResultData,
    pub transaction: Option<NovalnetCaptureTransactionData>,
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
            NovalnetAPIStatus::Success => {
                let transaction_id = item
                    .response
                    .transaction
                    .clone()
                    .map(|data| data.tid.expose().to_string());
                let transaction_status = item
                    .response
                    .transaction
                    .map(|transaction_data| transaction_data.status)
                    .unwrap_or(NovalnetTransactionStatus::Pending);

                Ok(Self {
                    status: common_enums::AttemptStatus::from(transaction_status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: transaction_id
                            .clone()
                            .map(ResponseId::ConnectorTransactionId)
                            .unwrap_or(ResponseId::NoResponseId),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id.clone(),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::Failure => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
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
            lang: item
                .request
                .get_optional_language_from_browser_info()
                .unwrap_or(consts::DEFAULT_LOCALE.to_string().to_string()),
        };
        Ok(Self {
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
            NovalnetAPIStatus::Success => {
                let refund_id = item
                    .response
                    .transaction
                    .clone()
                    .and_then(|data| data.tid)
                    .map(|tid| tid.expose().to_string())
                    .unwrap_or("".to_string());
                //NOTE: Mapping refund_id with "" incase we dont get any tid

                let transaction_status = item
                    .response
                    .transaction
                    .map(|transaction_data| transaction_data.status)
                    .unwrap_or(NovalnetTransactionStatus::Pending);

                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: refund_id,
                        refund_status: enums::RefundStatus::from(transaction_status),
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::Failure => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
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
            lang: item
                .request
                .get_optional_language_from_browser_info()
                .unwrap_or(consts::DEFAULT_LOCALE.to_string().to_string()),
        };
        Ok(Self {
            transaction,
            custom,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovalnetCancelResponse {
    result: ResultData,
    transaction: Option<NovalnetPaymentsResponseTransactionData>,
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
            NovalnetAPIStatus::Success => {
                let transaction_id = item
                    .response
                    .transaction
                    .clone()
                    .and_then(|data| data.tid.map(|tid| tid.expose().to_string()));
                let transaction_status = item
                    .response
                    .transaction
                    .and_then(|transaction_data| transaction_data.status)
                    .unwrap_or(NovalnetTransactionStatus::Pending);
                Ok(Self {
                    status: if transaction_status == NovalnetTransactionStatus::Deactivated {
                        enums::AttemptStatus::Voided
                    } else {
                        enums::AttemptStatus::VoidFailed
                    },
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: transaction_id
                            .clone()
                            .map(ResponseId::ConnectorTransactionId)
                            .unwrap_or(ResponseId::NoResponseId),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: transaction_id.clone(),
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            NovalnetAPIStatus::Failure => {
                let response = Err(get_error_response(item.response.result, item.http_code));
                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct NovalnetErrorResponse {
    pub status_code: u64,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Display, Debug, Serialize, Deserialize)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookEventType {
    Payment,
    TransactionCapture,
    TransactionCancel,
    TransactionRefund,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NovalnetWebhookEvent {
    pub checksum: String,
    pub tid: i64,
    pub parent_tid: Option<i64>,
    #[serde(rename = "type")]
    pub event_type: WebhookEventType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum NovalnetWebhookTransactionData {
    SyncTransactionData(NovalnetSyncResponseTransactionData),
    CaptureTransactionData(NovalnetCaptureTransactionData),
    CancelTransactionData(NovalnetPaymentsResponseTransactionData),
    RefundsTransactionData(NovalnetRefundsTransactionData),
}
#[derive(Serialize, Deserialize, Debug)]
pub struct NovalnetWebhookNotificationResponse {
    pub event: NovalnetWebhookEvent,
    pub result: ResultData,
    pub transaction: NovalnetWebhookTransactionData,
}

pub fn is_refund_event(event_code: &WebhookEventType) -> bool {
    matches!(event_code, WebhookEventType::TransactionRefund)
}

pub fn get_incoming_webhook_event(
    status: WebhookEventType,
    transaction_status: NovalnetTransactionStatus,
) -> IncomingWebhookEvent {
    match status {
        WebhookEventType::Payment => match transaction_status {
            NovalnetTransactionStatus::Confirmed | NovalnetTransactionStatus::Success => {
                IncomingWebhookEvent::PaymentIntentSuccess
            }
            NovalnetTransactionStatus::OnHold => {
                IncomingWebhookEvent::PaymentIntentAuthorizationSuccess
            }
            NovalnetTransactionStatus::Pending => IncomingWebhookEvent::PaymentIntentProcessing,
            NovalnetTransactionStatus::Progress => IncomingWebhookEvent::EventNotSupported,
            _ => IncomingWebhookEvent::PaymentIntentFailure,
        },
        WebhookEventType::TransactionCapture => match transaction_status {
            NovalnetTransactionStatus::Confirmed | NovalnetTransactionStatus::Success => {
                IncomingWebhookEvent::PaymentIntentCaptureSuccess
            }
            _ => IncomingWebhookEvent::PaymentIntentCaptureFailure,
        },
        WebhookEventType::TransactionCancel => match transaction_status {
            NovalnetTransactionStatus::Deactivated => IncomingWebhookEvent::PaymentIntentCancelled,
            _ => IncomingWebhookEvent::PaymentIntentCancelFailure,
        },
        WebhookEventType::TransactionRefund => match transaction_status {
            NovalnetTransactionStatus::Confirmed | NovalnetTransactionStatus::Success => {
                IncomingWebhookEvent::RefundSuccess
            }
            _ => IncomingWebhookEvent::RefundFailure,
        },
    }
}

pub fn reverse_string(s: &str) -> String {
    s.chars().rev().collect()
}
