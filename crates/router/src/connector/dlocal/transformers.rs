use api_models::payments::AddressDetails;
use common_utils::pii::Email;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{AddressDetailsData, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    services,
    types::{self, api, storage::enums},
};

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct Payer {
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
    pub document: Secret<String>,
}

#[derive(Debug, Default, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub holder_name: Secret<String>,
    pub number: cards::CardNumber,
    pub cvv: Secret<String>,
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
    pub capture: String,
    pub installments_id: Option<String>,
    pub installments: Option<String>,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct ThreeDSecureReqData {
    pub force: bool,
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentMethodId {
    #[default]
    Card,
}

#[derive(Debug, Serialize, Default, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentMethodFlow {
    #[default]
    Direct,
    ReDirect,
}

#[derive(Debug, Serialize)]
pub struct DlocalRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for DlocalRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, _currency, amount, router_data): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DlocalPaymentsRequest {
    pub amount: i64,
    pub currency: enums::Currency,
    pub country: String,
    pub payment_method_id: PaymentMethodId,
    pub payment_method_flow: PaymentMethodFlow,
    pub payer: Payer,
    pub card: Option<Card>,
    pub order_id: String,
    pub three_dsecure: Option<ThreeDSecureReqData>,
    pub callback_url: Option<String>,
    pub description: Option<String>,
}

impl TryFrom<&DlocalRouterData<&types::PaymentsAuthorizeRouterData>> for DlocalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DlocalRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.email.clone();
        let address = item.router_data.get_billing_address()?;
        let country = address.get_country()?;
        let name = get_payer_name(address);
        match item.router_data.request.payment_method_data {
            api::PaymentMethodData::Card(ref ccard) => {
                let should_capture = matches!(
                    item.router_data.request.capture_method,
                    Some(enums::CaptureMethod::Automatic)
                );
                let payment_request = Self {
                    amount: item.amount,
                    currency: item.router_data.request.currency,
                    payment_method_id: PaymentMethodId::Card,
                    payment_method_flow: PaymentMethodFlow::Direct,
                    country: country.to_string(),
                    payer: Payer {
                        name,
                        email,
                        // [#589]: Allow securely collecting PII from customer in payments request
                        document: get_doc_from_currency(country.to_string()),
                    },
                    card: Some(Card {
                        holder_name: ccard.card_holder_name.clone(),
                        number: ccard.card_number.clone(),
                        cvv: ccard.card_cvc.clone(),
                        expiration_month: ccard.card_exp_month.clone(),
                        expiration_year: ccard.card_exp_year.clone(),
                        capture: should_capture.to_string(),
                        installments_id: item
                            .router_data
                            .request
                            .mandate_id
                            .as_ref()
                            .map(|ids| ids.mandate_id.clone()),
                        // [#595[FEATURE] Pass Mandate history information in payment flows/request]
                        installments: item
                            .router_data
                            .request
                            .mandate_id
                            .clone()
                            .map(|_| "1".to_string()),
                    }),
                    order_id: item.router_data.connector_request_reference_id.clone(),
                    three_dsecure: match item.router_data.auth_type {
                        diesel_models::enums::AuthenticationType::ThreeDs => {
                            Some(ThreeDSecureReqData { force: true })
                        }
                        diesel_models::enums::AuthenticationType::NoThreeDs => None,
                    },
                    callback_url: Some(item.router_data.request.get_router_return_url()?),
                    description: item.router_data.description.clone(),
                };
                Ok(payment_request)
            }
            api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotImplemented(
                crate::connector::utils::get_unimplemented_payment_method_error_message("Dlocal"),
            ))?,
        }
    }
}

fn get_payer_name(address: &AddressDetails) -> Option<Secret<String>> {
    let first_name = address
        .first_name
        .clone()
        .map_or("".to_string(), |first_name| first_name.peek().to_string());
    let last_name = address
        .last_name
        .clone()
        .map_or("".to_string(), |last_name| last_name.peek().to_string());
    let name: String = format!("{first_name} {last_name}").trim().to_string();
    if !name.is_empty() {
        Some(Secret::new(name))
    } else {
        None
    }
}

pub struct DlocalPaymentsSyncRequest {
    pub authz_id: String,
}

impl TryFrom<&types::PaymentsSyncRouterData> for DlocalPaymentsSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            authz_id: (item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?),
        })
    }
}

pub struct DlocalPaymentsCancelRequest {
    pub cancel_id: String,
}

impl TryFrom<&types::PaymentsCancelRouterData> for DlocalPaymentsCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            cancel_id: item.request.connector_transaction_id.clone(),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DlocalPaymentsCaptureRequest {
    pub authorization_id: String,
    pub amount: i64,
    pub currency: String,
    pub order_id: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for DlocalPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            authorization_id: item.request.connector_transaction_id.clone(),
            amount: item.request.amount_to_capture,
            currency: item.request.currency.to_string(),
            order_id: item.connector_request_reference_id.clone(),
        })
    }
}
// Auth Struct
pub struct DlocalAuthType {
    pub(super) x_login: Secret<String>,
    pub(super) x_trans_key: Secret<String>,
    pub(super) secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for DlocalAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                x_login: api_key.to_owned(),
                x_trans_key: key1.to_owned(),
                secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
#[derive(Debug, Clone, Eq, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DlocalPaymentStatus {
    Authorized,
    Paid,
    Verified,
    Cancelled,
    #[default]
    Pending,
    Rejected,
}

impl From<DlocalPaymentStatus> for enums::AttemptStatus {
    fn from(item: DlocalPaymentStatus) -> Self {
        match item {
            DlocalPaymentStatus::Authorized => Self::Authorized,
            DlocalPaymentStatus::Verified => Self::Authorized,
            DlocalPaymentStatus::Paid => Self::Charged,
            DlocalPaymentStatus::Pending => Self::AuthenticationPending,
            DlocalPaymentStatus::Cancelled => Self::Voided,
            DlocalPaymentStatus::Rejected => Self::AuthenticationFailed,
        }
    }
}

#[derive(Eq, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreeDSecureResData {
    pub redirect_url: Option<Url>,
}

#[derive(Debug, Default, Eq, Clone, PartialEq, Serialize, Deserialize)]
pub struct DlocalPaymentsResponse {
    status: DlocalPaymentStatus,
    id: String,
    three_dsecure: Option<ThreeDSecureResData>,
    order_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .three_dsecure
            .and_then(|three_secure_data| three_secure_data.redirect_url)
            .map(|redirect_url| {
                services::RedirectForm::from((redirect_url, services::Method::Get))
            });

        let response = types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(item.response.order_id.clone()),
            redirection_data,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(item.response.order_id.clone()),
        };
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(response),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsSyncResponse {
    status: DlocalPaymentStatus,
    id: String,
    order_id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, DlocalPaymentsSyncResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            DlocalPaymentsSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id.clone()),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsCaptureResponse {
    status: DlocalPaymentStatus,
    id: String,
    order_id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, DlocalPaymentsCaptureResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            DlocalPaymentsCaptureResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id.clone()),
            }),
            ..item.data
        })
    }
}

pub struct DlocalPaymentsCancelResponse {
    status: DlocalPaymentStatus,
    order_id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, DlocalPaymentsCancelResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            DlocalPaymentsCancelResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.order_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id.clone()),
            }),
            ..item.data
        })
    }
}

// REFUND :
#[derive(Default, Debug, Serialize)]
pub struct DlocalRefundRequest {
    pub amount: String,
    pub payment_id: String,
    pub currency: enums::Currency,
    pub id: String,
}

impl<F> TryFrom<&DlocalRouterData<&types::RefundsRouterData<F>>> for DlocalRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DlocalRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let amount_to_refund = item.router_data.request.refund_amount.to_string();
        Ok(Self {
            amount: amount_to_refund,
            payment_id: item.router_data.request.connector_transaction_id.clone(),
            currency: item.router_data.request.currency,
            id: item.router_data.request.refund_id.clone(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Success,
    #[default]
    Pending,
    Rejected,
    Cancelled,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success => Self::Success,
            RefundStatus::Pending => Self::Pending,
            RefundStatus::Rejected => Self::ManualReview,
            RefundStatus::Cancelled => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DlocalRefundsSyncRequest {
    pub refund_id: String,
}

impl TryFrom<&types::RefundSyncRouterData> for DlocalRefundsSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let refund_id = match item.request.connector_refund_id.clone() {
            Some(val) => val,
            None => item.request.refund_id.clone(),
        };
        Ok(Self {
            refund_id: (refund_id),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DlocalErrorResponse {
    pub code: i32,
    pub message: String,
    pub param: Option<String>,
}

fn get_doc_from_currency(country: String) -> Secret<String> {
    let doc = match country.as_str() {
        "BR" => "91483309223",
        "ZA" => "2001014800086",
        "BD" | "GT" | "HN" | "PK" | "SN" | "TH" => "1234567890001",
        "CR" | "SV" | "VN" => "123456789",
        "DO" | "NG" => "12345678901",
        "EG" => "12345678901112",
        "GH" | "ID" | "RW" | "UG" => "1234567890111123",
        "IN" => "NHSTP6374G",
        "CI" => "CA124356789",
        "JP" | "MY" | "PH" => "123456789012",
        "NI" => "1234567890111A",
        "TZ" => "12345678912345678900",
        _ => "12345678",
    };
    Secret::new(doc.to_string())
}
