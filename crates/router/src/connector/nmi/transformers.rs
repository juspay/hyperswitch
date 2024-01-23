use api_models::webhooks;
use cards::CardNumber;
use common_utils::{errors::CustomResult, ext_traits::XmlExt};
use error_stack::{IntoReport, Report, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, RouterData,
    },
    core::errors,
    services,
    types::{self, api, storage::enums, transformers::ForeignFrom, ConnectorAuthType},
};

type Error = Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Auth,
    Capture,
    Refund,
    Sale,
    Validate,
    Void,
}

pub struct NmiAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) public_key: Option<Secret<String>>,
}

impl TryFrom<&ConnectorAuthType> for NmiAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
                public_key: None,
            }),
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                public_key: Some(key1.to_owned()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NmiRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for NmiRouterData<T>
{
    type Error = Report<errors::ConnectorError>;

    fn try_from(
        (_currency_unit, currency, amount, router_data): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: utils::to_currency_base_unit_asf64(amount, currency)?,
            router_data,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiVaultRequest {
    security_key: Secret<String>,
    ccnumber: CardNumber,
    ccexp: Secret<String>,
    cvv: Secret<String>,
    first_name: Secret<String>,
    last_name: Secret<String>,
    customer_vault: CustomerAction,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomerAction {
    AddCustomer,
    UpdateCustomer,
}

impl TryFrom<&types::PaymentsPreProcessingRouterData> for NmiVaultRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
        let auth_type: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let (ccnumber, ccexp, cvv) = get_card_details(item.request.payment_method_data.clone())?;
        let billing_details = item.get_billing_address()?;

        Ok(Self {
            security_key: auth_type.api_key,
            ccnumber,
            ccexp,
            cvv,
            first_name: billing_details.get_first_name()?.to_owned(),
            last_name: billing_details.get_last_name()?.to_owned(),
            customer_vault: CustomerAction::AddCustomer,
        })
    }
}

fn get_card_details(
    payment_method_data: Option<api::PaymentMethodData>,
) -> CustomResult<(CardNumber, Secret<String>, Secret<String>), errors::ConnectorError> {
    match payment_method_data {
        Some(api::PaymentMethodData::Card(ref card_details)) => Ok((
            card_details.card_number.clone(),
            utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
                card_details,
                "".to_string(),
            )?,
            card_details.card_cvc.clone(),
        )),
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("Nmi"),
        ))
        .into_report(),
    }
}

#[derive(Debug, Deserialize)]
pub struct NmiVaultResponse {
    pub response: Response,
    pub responsetext: String,
    pub customer_vault_id: Option<String>,
    pub response_code: String,
    pub transactionid: String,
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::PreProcessing,
            NmiVaultResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    > for types::PaymentsPreProcessingRouterData
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::PreProcessing,
            NmiVaultResponse,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let auth_type: NmiAuthType = (&item.data.connector_auth_type).try_into()?;
        let amount_data =
            item.data
                .request
                .amount
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "amount",
                })?;
        let currency_data =
            item.data
                .request
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::NoResponseId,
                    redirection_data: Some(services::RedirectForm::Nmi {
                        amount: utils::to_currency_base_unit_asf64(
                            amount_data,
                            currency_data.to_owned(),
                        )?
                        .to_string(),
                        currency: currency_data,
                        customer_vault_id: item.response.customer_vault_id.ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "customer_vault_id",
                            },
                        )?,
                        public_key: auth_type.public_key.ok_or(
                            errors::ConnectorError::InvalidConnectorConfig {
                                config: "public_key",
                            },
                        )?,
                        order_id: item.data.connector_request_reference_id.clone(),
                    }),
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.transactionid),
                    incremental_authorization_allowed: None,
                }),
                enums::AttemptStatus::AuthenticationPending,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.responsetext.to_owned(),
                    reason: Some(item.response.responsetext),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transactionid),
                }),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiCompleteRequest {
    amount: f64,
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    security_key: Secret<String>,
    orderid: String,
    ccnumber: CardNumber,
    ccexp: Secret<String>,
    cardholder_auth: CardHolderAuthType,
    cavv: String,
    xid: String,
    three_ds_version: Option<ThreeDsVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardHolderAuthType {
    Verified,
    Attempted,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ThreeDsVersion {
    #[serde(rename = "2.0.0")]
    VersionTwo,
    #[serde(rename = "2.1.0")]
    VersionTwoPointOne,
    #[serde(rename = "2.2.0")]
    VersionTwoPointTwo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NmiRedirectResponseData {
    cavv: String,
    xid: String,
    card_holder_auth: CardHolderAuthType,
    three_ds_version: Option<ThreeDsVersion>,
    order_id: String,
}

impl TryFrom<&NmiRouterData<&types::PaymentsCompleteAuthorizeRouterData>> for NmiCompleteRequest {
    type Error = Error;
    fn try_from(
        item: &NmiRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let transaction_type = match item.router_data.request.is_auto_capture()? {
            true => TransactionType::Sale,
            false => TransactionType::Auth,
        };
        let auth_type: NmiAuthType = (&item.router_data.connector_auth_type).try_into()?;
        let payload_data = item
            .router_data
            .request
            .get_redirect_response_payload()?
            .expose();

        let three_ds_data: NmiRedirectResponseData = serde_json::from_value(payload_data)
            .into_report()
            .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "three_ds_data",
            })?;

        let (ccnumber, ccexp, ..) =
            get_card_details(item.router_data.request.payment_method_data.clone())?;

        Ok(Self {
            amount: item.amount,
            transaction_type,
            security_key: auth_type.api_key,
            orderid: three_ds_data.order_id,
            ccnumber,
            ccexp,
            cardholder_auth: three_ds_data.card_holder_auth,
            cavv: three_ds_data.cavv,
            xid: three_ds_data.xid,
            three_ds_version: three_ds_data.three_ds_version,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct NmiCompleteResponse {
    pub response: Response,
    pub responsetext: String,
    pub authcode: Option<String>,
    pub transactionid: String,
    pub avsresponse: Option<String>,
    pub cvvresponse: Option<String>,
    pub orderid: String,
    pub response_code: String,
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::CompleteAuthorize,
            NmiCompleteResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::PaymentsCompleteAuthorizeRouterData
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::CompleteAuthorize,
            NmiCompleteResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                }),
                if let Some(diesel_models::enums::CaptureMethod::Automatic) =
                    item.data.request.capture_method
                {
                    enums::AttemptStatus::CaptureInitiated
                } else {
                    enums::AttemptStatus::Authorizing
                },
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl ForeignFrom<(NmiCompleteResponse, u16)> for types::ErrorResponse {
    fn foreign_from((response, http_code): (NmiCompleteResponse, u16)) -> Self {
        Self {
            code: response.response_code,
            message: response.responsetext.to_owned(),
            reason: Some(response.responsetext),
            status_code: http_code,
            attempt_status: None,
            connector_transaction_id: Some(response.transactionid),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    amount: f64,
    security_key: Secret<String>,
    currency: enums::Currency,
    #[serde(flatten)]
    payment_method: PaymentMethod,
    orderid: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentMethod {
    Card(Box<CardData>),
    GPay(Box<GooglePayData>),
    ApplePay(Box<ApplePayData>),
}

#[derive(Debug, Serialize)]
pub struct CardData {
    ccnumber: CardNumber,
    ccexp: Secret<String>,
    cvv: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct GooglePayData {
    googlepay_payment_data: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct ApplePayData {
    applepay_payment_data: Secret<String>,
}

impl TryFrom<&NmiRouterData<&types::PaymentsAuthorizeRouterData>> for NmiPaymentsRequest {
    type Error = Error;
    fn try_from(
        item: &NmiRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let transaction_type = match item.router_data.request.is_auto_capture()? {
            true => TransactionType::Sale,
            false => TransactionType::Auth,
        };
        let auth_type: NmiAuthType = (&item.router_data.connector_auth_type).try_into()?;
        let amount = item.amount;
        let payment_method =
            PaymentMethod::try_from(&item.router_data.request.payment_method_data)?;

        Ok(Self {
            transaction_type,
            security_key: auth_type.api_key,
            amount,
            currency: item.router_data.request.currency,
            payment_method,
            orderid: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

impl TryFrom<&api_models::payments::PaymentMethodData> for PaymentMethod {
    type Error = Error;
    fn try_from(
        payment_method_data: &api_models::payments::PaymentMethodData,
    ) -> Result<Self, Self::Error> {
        match &payment_method_data {
            api::PaymentMethodData::Card(ref card) => Ok(Self::try_from(card)?),
            api::PaymentMethodData::Wallet(ref wallet_type) => match wallet_type {
                api_models::payments::WalletData::GooglePay(ref googlepay_data) => {
                    Ok(Self::from(googlepay_data))
                }
                api_models::payments::WalletData::ApplePay(ref applepay_data) => {
                    Ok(Self::from(applepay_data))
                }
                api_models::payments::WalletData::AliPayQr(_)
                | api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::AliPayHkRedirect(_)
                | api_models::payments::WalletData::MomoRedirect(_)
                | api_models::payments::WalletData::KakaoPayRedirect(_)
                | api_models::payments::WalletData::GoPayRedirect(_)
                | api_models::payments::WalletData::GcashRedirect(_)
                | api_models::payments::WalletData::ApplePayRedirect(_)
                | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                | api_models::payments::WalletData::DanaRedirect {}
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
                    Err(errors::ConnectorError::NotSupported {
                        message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                        connector: "nmi",
                    })
                    .into_report()
                }
            },
            api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_)
            | api::PaymentMethodData::CardToken(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "nmi",
            })
            .into_report(),
        }
    }
}

impl TryFrom<&api_models::payments::Card> for PaymentMethod {
    type Error = Error;
    fn try_from(card: &api_models::payments::Card) -> Result<Self, Self::Error> {
        let ccexp = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
            card,
            "".to_string(),
        )?;
        let card = CardData {
            ccnumber: card.card_number.clone(),
            ccexp,
            cvv: card.card_cvc.clone(),
        };
        Ok(Self::Card(Box::new(card)))
    }
}

impl From<&api_models::payments::GooglePayWalletData> for PaymentMethod {
    fn from(wallet_data: &api_models::payments::GooglePayWalletData) -> Self {
        let gpay_data = GooglePayData {
            googlepay_payment_data: Secret::new(wallet_data.tokenization_data.token.clone()),
        };
        Self::GPay(Box::new(gpay_data))
    }
}

impl From<&api_models::payments::ApplePayWalletData> for PaymentMethod {
    fn from(wallet_data: &api_models::payments::ApplePayWalletData) -> Self {
        let apple_pay_data = ApplePayData {
            applepay_payment_data: Secret::new(wallet_data.payment_data.clone()),
        };
        Self::ApplePay(Box::new(apple_pay_data))
    }
}

impl TryFrom<&types::SetupMandateRouterData> for NmiPaymentsRequest {
    type Error = Error;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        let auth_type: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let payment_method = PaymentMethod::try_from(&item.request.payment_method_data)?;
        Ok(Self {
            transaction_type: TransactionType::Validate,
            security_key: auth_type.api_key,
            amount: 0.0,
            currency: item.request.currency,
            payment_method,
            orderid: item.connector_request_reference_id.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiSyncRequest {
    pub order_id: String,
    pub security_key: Secret<String>,
}

impl TryFrom<&types::PaymentsSyncRouterData> for NmiSyncRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            security_key: auth.api_key,
            order_id: item.attempt_id.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiCaptureRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: Secret<String>,
    pub transactionid: String,
    pub amount: Option<f64>,
}

impl TryFrom<&NmiRouterData<&types::PaymentsCaptureRouterData>> for NmiCaptureRequest {
    type Error = Error;
    fn try_from(
        item: &NmiRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            transaction_type: TransactionType::Capture,
            security_key: auth.api_key,
            transactionid: item.router_data.request.connector_transaction_id.clone(),
            amount: Some(item.amount),
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::Capture,
            StandardResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::Capture,
            StandardResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid.to_owned(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                }),
                enums::AttemptStatus::CaptureInitiated,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::CaptureFailed,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiCancelRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: Secret<String>,
    pub transactionid: String,
    pub void_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for NmiCancelRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            transaction_type: TransactionType::Void,
            security_key: auth.api_key,
            transactionid: item.request.connector_transaction_id.clone(),
            void_reason: item.request.cancellation_reason.clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub enum Response {
    #[serde(alias = "1")]
    Approved,
    #[serde(alias = "2")]
    Declined,
    #[serde(alias = "3")]
    Error,
}

#[derive(Debug, Deserialize)]
pub struct StandardResponse {
    pub response: Response,
    pub responsetext: String,
    pub authcode: Option<String>,
    pub transactionid: String,
    pub avsresponse: Option<String>,
    pub cvvresponse: Option<String>,
    pub orderid: String,
    pub response_code: String,
}

impl<T>
    TryFrom<
        types::ResponseRouterData<
            api::SetupMandate,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<api::SetupMandate, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::SetupMandate,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid.to_owned(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                }),
                enums::AttemptStatus::Charged,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl ForeignFrom<(StandardResponse, u16)> for types::ErrorResponse {
    fn foreign_from((response, http_code): (StandardResponse, u16)) -> Self {
        Self {
            code: response.response_code,
            message: response.responsetext.to_owned(),
            reason: Some(response.responsetext),
            status_code: http_code,
            attempt_status: None,
            connector_transaction_id: Some(response.transactionid),
        }
    }
}

impl TryFrom<types::PaymentsResponseRouterData<StandardResponse>>
    for types::RouterData<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authorize,
            StandardResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid.to_owned(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                }),
                if let Some(diesel_models::enums::CaptureMethod::Automatic) =
                    item.data.request.capture_method
                {
                    enums::AttemptStatus::CaptureInitiated
                } else {
                    enums::AttemptStatus::Authorizing
                },
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<T>
    TryFrom<types::ResponseRouterData<api::Void, StandardResponse, T, types::PaymentsResponseData>>
    for types::RouterData<api::Void, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            api::Void,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid.to_owned(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                }),
                enums::AttemptStatus::VoidInitiated,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse::foreign_from((
                    item.response,
                    item.http_code,
                ))),
                enums::AttemptStatus::VoidFailed,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NmiStatus {
    Abandoned,
    Cancelled,
    Pendingsettlement,
    Pending,
    Failed,
    Complete,
    InProgress,
    Unknown,
}

impl TryFrom<types::PaymentsSyncResponseRouterData<types::Response>>
    for types::PaymentsSyncRouterData
{
    type Error = Error;
    fn try_from(
        item: types::PaymentsSyncResponseRouterData<types::Response>,
    ) -> Result<Self, Self::Error> {
        let response = SyncResponse::try_from(item.response.response.to_vec())?;
        Ok(Self {
            status: enums::AttemptStatus::from(NmiStatus::from(response.transaction.condition)),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    response.transaction.transaction_id,
                ),
                redirection_data: None,
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

impl TryFrom<Vec<u8>> for SyncResponse {
    type Error = Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let query_response = String::from_utf8(bytes)
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        query_response
            .parse_xml::<Self>()
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
}

impl TryFrom<Vec<u8>> for NmiRefundSyncResponse {
    type Error = Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let query_response = String::from_utf8(bytes)
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        query_response
            .parse_xml::<Self>()
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
}

impl From<NmiStatus> for enums::AttemptStatus {
    fn from(item: NmiStatus) -> Self {
        match item {
            NmiStatus::Abandoned => Self::AuthenticationFailed,
            NmiStatus::Cancelled => Self::Voided,
            NmiStatus::Pending => Self::Authorized,
            NmiStatus::Pendingsettlement | NmiStatus::Complete => Self::Charged,
            NmiStatus::InProgress => Self::AuthenticationPending,
            NmiStatus::Failed | NmiStatus::Unknown => Self::Failure,
        }
    }
}

// REFUND :
#[derive(Debug, Serialize)]
pub struct NmiRefundRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    security_key: Secret<String>,
    transactionid: String,
    orderid: String,
    amount: f64,
}

impl<F> TryFrom<&NmiRouterData<&types::RefundsRouterData<F>>> for NmiRefundRequest {
    type Error = Error;
    fn try_from(item: &NmiRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type: NmiAuthType = (&item.router_data.connector_auth_type).try_into()?;
        Ok(Self {
            transaction_type: TransactionType::Refund,
            security_key: auth_type.api_key,
            transactionid: item.router_data.request.connector_transaction_id.clone(),
            orderid: item.router_data.request.refund_id.clone(),
            amount: item.amount,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, StandardResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.orderid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Capture, StandardResponse>>
    for types::RefundsRouterData<api::Capture>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Capture, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transactionid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<Response> for enums::RefundStatus {
    fn from(item: Response) -> Self {
        match item {
            Response::Approved => Self::Pending,
            Response::Declined | Response::Error => Self::Failure,
        }
    }
}

impl TryFrom<&types::RefundSyncRouterData> for NmiSyncRequest {
    type Error = Error;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            security_key: auth.api_key,
            order_id: item
                .request
                .connector_refund_id
                .clone()
                .ok_or(errors::ConnectorError::MissingConnectorRefundID)?,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, types::Response>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, types::Response>,
    ) -> Result<Self, Self::Error> {
        let response = NmiRefundSyncResponse::try_from(item.response.response.to_vec())?;
        let refund_status =
            enums::RefundStatus::from(NmiStatus::from(response.transaction.condition));
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: response.transaction.order_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<NmiStatus> for enums::RefundStatus {
    fn from(item: NmiStatus) -> Self {
        match item {
            NmiStatus::Abandoned
            | NmiStatus::Cancelled
            | NmiStatus::Failed
            | NmiStatus::Unknown => Self::Failure,
            NmiStatus::Pending | NmiStatus::InProgress => Self::Pending,
            NmiStatus::Pendingsettlement | NmiStatus::Complete => Self::Success,
        }
    }
}

impl From<String> for NmiStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "abandoned" => Self::Abandoned,
            "canceled" => Self::Cancelled,
            "in_progress" => Self::InProgress,
            "pendingsettlement" => Self::Pendingsettlement,
            "complete" => Self::Complete,
            "failed" => Self::Failed,
            "unknown" => Self::Unknown,
            // Other than above values only pending is possible, since value is a string handling this as default
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SyncTransactionResponse {
    pub transaction_id: String,
    pub condition: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SyncResponse {
    pub transaction: SyncTransactionResponse,
}

#[derive(Debug, Deserialize)]
pub struct RefundSyncBody {
    order_id: String,
    condition: String,
}

#[derive(Debug, Deserialize)]
struct NmiRefundSyncResponse {
    transaction: RefundSyncBody,
}

#[derive(Debug, Deserialize)]
pub struct NmiWebhookObjectReference {
    pub event_body: NmiReferenceBody,
}

#[derive(Debug, Deserialize)]
pub struct NmiReferenceBody {
    pub order_id: String,
    pub action: NmiActionBody,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NmiActionBody {
    pub action_type: NmiActionType,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NmiActionType {
    Auth,
    Capture,
    Credit,
    Refund,
    Sale,
    Void,
}

#[derive(Debug, Deserialize)]
pub struct NmiWebhookEventBody {
    pub event_type: NmiWebhookEventType,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum NmiWebhookEventType {
    #[serde(rename = "transaction.sale.success")]
    SaleSuccess,
    #[serde(rename = "transaction.sale.failure")]
    SaleFailure,
    #[serde(rename = "transaction.sale.unknown")]
    SaleUnknown,
    #[serde(rename = "transaction.auth.success")]
    AuthSuccess,
    #[serde(rename = "transaction.auth.failure")]
    AuthFailure,
    #[serde(rename = "transaction.auth.unknown")]
    AuthUnknown,
    #[serde(rename = "transaction.refund.success")]
    RefundSuccess,
    #[serde(rename = "transaction.refund.failure")]
    RefundFailure,
    #[serde(rename = "transaction.refund.unknown")]
    RefundUnknown,
    #[serde(rename = "transaction.void.success")]
    VoidSuccess,
    #[serde(rename = "transaction.void.failure")]
    VoidFailure,
    #[serde(rename = "transaction.void.unknown")]
    VoidUnknown,
    #[serde(rename = "transaction.capture.success")]
    CaptureSuccess,
    #[serde(rename = "transaction.capture.failure")]
    CaptureFailure,
    #[serde(rename = "transaction.capture.unknown")]
    CaptureUnknown,
}

impl ForeignFrom<NmiWebhookEventType> for webhooks::IncomingWebhookEvent {
    fn foreign_from(status: NmiWebhookEventType) -> Self {
        match status {
            NmiWebhookEventType::SaleSuccess => Self::PaymentIntentSuccess,
            NmiWebhookEventType::SaleFailure => Self::PaymentIntentFailure,
            NmiWebhookEventType::RefundSuccess => Self::RefundSuccess,
            NmiWebhookEventType::RefundFailure => Self::RefundFailure,
            NmiWebhookEventType::VoidSuccess => Self::PaymentIntentCancelled,
            NmiWebhookEventType::AuthSuccess => Self::PaymentIntentAuthorizationSuccess,
            NmiWebhookEventType::CaptureSuccess => Self::PaymentIntentCaptureSuccess,
            NmiWebhookEventType::AuthFailure => Self::PaymentIntentAuthorizationFailure,
            NmiWebhookEventType::CaptureFailure => Self::PaymentIntentCaptureFailure,
            NmiWebhookEventType::VoidFailure => Self::PaymentIntentCancelFailure,
            NmiWebhookEventType::SaleUnknown
            | NmiWebhookEventType::RefundUnknown
            | NmiWebhookEventType::AuthUnknown
            | NmiWebhookEventType::VoidUnknown
            | NmiWebhookEventType::CaptureUnknown => Self::EventNotSupported,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NmiWebhookBody {
    pub event_body: NmiWebhookObject,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NmiWebhookObject {
    pub transaction_id: String,
    pub order_id: String,
    pub condition: String,
    pub action: NmiActionBody,
}

impl TryFrom<&NmiWebhookBody> for SyncResponse {
    type Error = Error;
    fn try_from(item: &NmiWebhookBody) -> Result<Self, Self::Error> {
        let transaction = SyncTransactionResponse {
            transaction_id: item.event_body.transaction_id.to_owned(),
            condition: item.event_body.condition.to_owned(),
        };

        Ok(Self { transaction })
    }
}
