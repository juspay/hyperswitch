use api_models::webhooks;
use cards::CardNumber;
use common_enums::CountryAlpha2;
use common_utils::{
    errors::CustomResult,
    ext_traits::XmlExt,
    pii::{self, Email},
};
use error_stack::{report, Report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, RouterData,
    },
    core::errors,
    services,
    types::{self, api, domain, storage::enums, transformers::ForeignFrom, ConnectorAuthType},
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
    address1: Option<Secret<String>>,
    address2: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    zip: Option<Secret<String>>,
    country: Option<CountryAlpha2>,
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
        let first_name = billing_details.get_first_name()?;

        Ok(Self {
            security_key: auth_type.api_key,
            ccnumber,
            ccexp,
            cvv,
            first_name: first_name.clone(),
            last_name: billing_details
                .get_last_name()
                .unwrap_or(first_name)
                .clone(),
            address1: billing_details.line1.clone(),
            address2: billing_details.line2.clone(),
            city: billing_details.city.clone(),
            state: billing_details.state.clone(),
            country: billing_details.country,
            zip: billing_details.zip.clone(),
            customer_vault: CustomerAction::AddCustomer,
        })
    }
}

fn get_card_details(
    payment_method_data: Option<domain::PaymentMethodData>,
) -> CustomResult<(CardNumber, Secret<String>, Secret<String>), errors::ConnectorError> {
    match payment_method_data {
        Some(domain::PaymentMethodData::Card(ref card_details)) => Ok((
            card_details.card_number.clone(),
            utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
                card_details,
                "".to_string(),
            )?,
            card_details.card_cvc.clone(),
        )),
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("Nmi"),
        )
        .into()),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NmiVaultResponse {
    pub response: Response,
    pub responsetext: String,
    pub customer_vault_id: Option<Secret<String>>,
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
                        customer_vault_id: item
                            .response
                            .customer_vault_id
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "customer_vault_id",
                            })?
                            .peek()
                            .to_string(),
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
    orderid: Option<String>,
    customer_vault_id: Secret<String>,
    email: Option<Email>,
    cardholder_auth: Option<String>,
    cavv: Option<String>,
    xid: Option<String>,
    eci: Option<String>,
    cvv: Secret<String>,
    three_ds_version: Option<String>,
    directory_server_id: Option<Secret<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum NmiRedirectResponse {
    NmiRedirectResponseData(NmiRedirectResponseData),
    NmiErrorResponseData(NmiErrorResponseData),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NmiErrorResponseData {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NmiRedirectResponseData {
    cavv: Option<String>,
    xid: Option<String>,
    eci: Option<String>,
    card_holder_auth: Option<String>,
    three_ds_version: Option<String>,
    order_id: Option<String>,
    directory_server_id: Option<Secret<String>>,
    customer_vault_id: Secret<String>,
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
            .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "three_ds_data",
            })?;

        let (_, _, cvv) = get_card_details(item.router_data.request.payment_method_data.clone())?;

        Ok(Self {
            amount: item.amount,
            transaction_type,
            security_key: auth_type.api_key,
            orderid: three_ds_data.order_id,
            customer_vault_id: three_ds_data.customer_vault_id,
            email: item.router_data.request.email.clone(),
            cvv,
            cardholder_auth: three_ds_data.card_holder_auth,
            cavv: three_ds_data.cavv,
            xid: three_ds_data.xid,
            eci: three_ds_data.eci,
            three_ds_version: three_ds_data.three_ds_version,
            directory_server_id: three_ds_data.directory_server_id,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
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
    #[serde(flatten)]
    merchant_defined_field: Option<NmiMerchantDefinedField>,
    orderid: String,
}

#[derive(Debug, Serialize)]
pub struct NmiMerchantDefinedField {
    #[serde(flatten)]
    inner: std::collections::BTreeMap<String, Secret<String>>,
}

impl NmiMerchantDefinedField {
    pub fn new(metadata: &pii::SecretSerdeValue) -> Self {
        let metadata_as_string = metadata.peek().to_string();
        let hash_map: std::collections::BTreeMap<String, serde_json::Value> =
            serde_json::from_str(&metadata_as_string).unwrap_or(std::collections::BTreeMap::new());
        let inner = hash_map
            .into_iter()
            .enumerate()
            .map(|(index, (hs_key, hs_value))| {
                let nmi_key = format!("merchant_defined_field_{}", index + 1);
                let nmi_value = format!("{hs_key}={hs_value}");
                (nmi_key, Secret::new(nmi_value))
            })
            .collect();
        Self { inner }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentMethod {
    CardNonThreeDs(Box<CardData>),
    CardThreeDs(Box<CardThreeDsData>),
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
pub struct CardThreeDsData {
    ccnumber: CardNumber,
    ccexp: Secret<String>,
    email: Option<Email>,
    cardholder_auth: Option<String>,
    cavv: Option<String>,
    eci: Option<String>,
    cvv: Secret<String>,
    three_ds_version: Option<String>,
    directory_server_id: Option<Secret<String>>,
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
        let payment_method = PaymentMethod::try_from((
            &item.router_data.request.payment_method_data,
            Some(item.router_data),
        ))?;

        Ok(Self {
            transaction_type,
            security_key: auth_type.api_key,
            amount,
            currency: item.router_data.request.currency,
            payment_method,
            merchant_defined_field: item
                .router_data
                .request
                .metadata
                .as_ref()
                .map(NmiMerchantDefinedField::new),
            orderid: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

impl
    TryFrom<(
        &domain::PaymentMethodData,
        Option<&types::PaymentsAuthorizeRouterData>,
    )> for PaymentMethod
{
    type Error = Error;
    fn try_from(
        item: (
            &domain::PaymentMethodData,
            Option<&types::PaymentsAuthorizeRouterData>,
        ),
    ) -> Result<Self, Self::Error> {
        let (payment_method_data, router_data) = item;
        match payment_method_data {
            domain::PaymentMethodData::Card(ref card) => match router_data {
                Some(data) => match data.auth_type {
                    common_enums::AuthenticationType::NoThreeDs => Ok(Self::try_from(card)?),
                    common_enums::AuthenticationType::ThreeDs => {
                        Ok(Self::try_from((card, &data.request))?)
                    }
                },
                None => Ok(Self::try_from(card)?),
            },
            domain::PaymentMethodData::Wallet(ref wallet_type) => match wallet_type {
                domain::WalletData::GooglePay(ref googlepay_data) => Ok(Self::from(googlepay_data)),
                domain::WalletData::ApplePay(ref applepay_data) => Ok(Self::from(applepay_data)),
                domain::WalletData::AliPayQr(_)
                | domain::WalletData::AliPayRedirect(_)
                | domain::WalletData::AliPayHkRedirect(_)
                | domain::WalletData::MomoRedirect(_)
                | domain::WalletData::KakaoPayRedirect(_)
                | domain::WalletData::GoPayRedirect(_)
                | domain::WalletData::GcashRedirect(_)
                | domain::WalletData::ApplePayRedirect(_)
                | domain::WalletData::ApplePayThirdPartySdk(_)
                | domain::WalletData::DanaRedirect {}
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
                | domain::WalletData::WeChatPayQr(_)
                | domain::WalletData::CashappQr(_)
                | domain::WalletData::SwishQr(_) => {
                    Err(report!(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("nmi"),
                    )))
                }
            },
            domain::PaymentMethodData::CardRedirect(_)
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
                    utils::get_unimplemented_payment_method_error_message("nmi"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<(&domain::payments::Card, &types::PaymentsAuthorizeData)> for PaymentMethod {
    type Error = Error;
    fn try_from(
        val: (&domain::payments::Card, &types::PaymentsAuthorizeData),
    ) -> Result<Self, Self::Error> {
        let (card_data, item) = val;
        let auth_data = &item.get_authentication_data()?;
        let ccexp = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
            card_data,
            "".to_string(),
        )?;

        let card_3ds_details = CardThreeDsData {
            ccnumber: card_data.card_number.clone(),
            ccexp,
            cvv: card_data.card_cvc.clone(),
            email: item.email.clone(),
            cavv: Some(auth_data.cavv.clone()),
            eci: auth_data.eci.clone(),
            cardholder_auth: None,
            three_ds_version: Some(auth_data.message_version.clone()),
            directory_server_id: Some(auth_data.threeds_server_transaction_id.clone().into()),
        };

        Ok(Self::CardThreeDs(Box::new(card_3ds_details)))
    }
}

impl TryFrom<&domain::payments::Card> for PaymentMethod {
    type Error = Error;
    fn try_from(card: &domain::payments::Card) -> Result<Self, Self::Error> {
        let ccexp = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
            card,
            "".to_string(),
        )?;
        let card = CardData {
            ccnumber: card.card_number.clone(),
            ccexp,
            cvv: card.card_cvc.clone(),
        };
        Ok(Self::CardNonThreeDs(Box::new(card)))
    }
}

impl From<&domain::GooglePayWalletData> for PaymentMethod {
    fn from(wallet_data: &domain::GooglePayWalletData) -> Self {
        let gpay_data = GooglePayData {
            googlepay_payment_data: Secret::new(wallet_data.tokenization_data.token.clone()),
        };
        Self::GPay(Box::new(gpay_data))
    }
}

impl From<&domain::ApplePayWalletData> for PaymentMethod {
    fn from(wallet_data: &domain::ApplePayWalletData) -> Self {
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
        let payment_method = PaymentMethod::try_from((&item.request.payment_method_data, None))?;
        Ok(Self {
            transaction_type: TransactionType::Validate,
            security_key: auth_type.api_key,
            amount: 0.0,
            currency: item.request.currency,
            payment_method,
            merchant_defined_field: None,
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

#[derive(Debug, Deserialize, Serialize)]
pub enum Response {
    #[serde(alias = "1")]
    Approved,
    #[serde(alias = "2")]
    Declined,
    #[serde(alias = "3")]
    Error,
}

#[derive(Debug, Deserialize, Serialize)]
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
                        item.response.transactionid.clone(),
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
                        item.response.transactionid.clone(),
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
                        item.response.transactionid.clone(),
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

impl<F, T> TryFrom<types::ResponseRouterData<F, SyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<F, SyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(trn) => Ok(Self {
                status: enums::AttemptStatus::from(NmiStatus::from(trn.condition)),
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(trn.transaction_id),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                }),
                ..item.data
            }),
            None => Ok(Self { ..item.data }), //when there is empty connector response i.e. response we get in psync when payment status is in authentication_pending
        }
    }
}

impl TryFrom<Vec<u8>> for SyncResponse {
    type Error = Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let query_response = String::from_utf8(bytes)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        query_response
            .parse_xml::<Self>()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)
    }
}

impl TryFrom<Vec<u8>> for NmiRefundSyncResponse {
    type Error = Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let query_response = String::from_utf8(bytes)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        query_response
            .parse_xml::<Self>()
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, NmiRefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, NmiRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status =
            enums::RefundStatus::from(NmiStatus::from(item.response.transaction.condition));
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction.order_id,
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
    pub transaction: Option<SyncTransactionResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RefundSyncBody {
    order_id: String,
    condition: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NmiRefundSyncResponse {
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
        let transaction = Some(SyncTransactionResponse {
            transaction_id: item.event_body.transaction_id.to_owned(),
            condition: item.event_body.condition.to_owned(),
        });

        Ok(Self { transaction })
    }
}
