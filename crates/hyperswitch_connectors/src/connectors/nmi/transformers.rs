use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use cards::CardNumber;
use common_enums::{AttemptStatus, AuthenticationType, CountryAlpha2, Currency, RefundStatus};
use common_utils::{errors::CustomResult, ext_traits::XmlExt, pii::Email, types::FloatMajorUnit};
use error_stack::{report, Report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{
        ApplePayWalletData, Card, GooglePayWalletData, PaymentMethodData, WalletData,
    },
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        Authorize, Capture, CompleteAuthorize, Execute, RSync, SetupMandate, Void,
    },
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCaptureData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsPreprocessingResponseRouterData, PaymentsResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        get_unimplemented_payment_method_error_message, to_currency_base_unit_asf64,
        AddressDetailsData as _, CardData as _, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData as _, RouterData as _,
    },
};

type Error = Report<ConnectorError>;

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
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
                public_key: None,
            }),
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                public_key: Some(key1.to_owned()),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NmiRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for NmiRouterData<T> {
    fn from((amount, router_data): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
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

impl TryFrom<&PaymentsPreProcessingRouterData> for NmiVaultRequest {
    type Error = Error;
    fn try_from(item: &PaymentsPreProcessingRouterData) -> Result<Self, Self::Error> {
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
    payment_method_data: Option<PaymentMethodData>,
) -> CustomResult<(CardNumber, Secret<String>, Secret<String>), ConnectorError> {
    match payment_method_data {
        Some(PaymentMethodData::Card(ref card_details)) => Ok((
            card_details.card_number.clone(),
            card_details.get_card_expiry_month_year_2_digit_with_delimiter("".to_string())?,
            card_details.card_cvc.clone(),
        )),
        _ => Err(
            ConnectorError::NotImplemented(get_unimplemented_payment_method_error_message("Nmi"))
                .into(),
        ),
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

impl TryFrom<PaymentsPreprocessingResponseRouterData<NmiVaultResponse>>
    for PaymentsPreProcessingRouterData
{
    type Error = Error;
    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<NmiVaultResponse>,
    ) -> Result<Self, Self::Error> {
        let auth_type: NmiAuthType = (&item.data.connector_auth_type).try_into()?;
        let amount_data = item
            .data
            .request
            .amount
            .ok_or(ConnectorError::MissingRequiredField {
                field_name: "amount",
            })?;
        let currency_data =
            item.data
                .request
                .currency
                .ok_or(ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(Some(RedirectForm::Nmi {
                        amount: to_currency_base_unit_asf64(amount_data, currency_data.to_owned())?
                            .to_string(),
                        currency: currency_data,
                        customer_vault_id: item
                            .response
                            .customer_vault_id
                            .ok_or(ConnectorError::MissingRequiredField {
                                field_name: "customer_vault_id",
                            })?
                            .peek()
                            .to_string(),
                        public_key: auth_type.public_key.ok_or(
                            ConnectorError::InvalidConnectorConfig {
                                config: "public_key",
                            },
                        )?,
                        order_id: item.data.connector_request_reference_id.clone(),
                    })),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.transactionid),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                AttemptStatus::AuthenticationPending,
            ),
            Response::Declined | Response::Error => (
                Err(ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.responsetext.to_owned(),
                    reason: Some(item.response.responsetext),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transactionid),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                AttemptStatus::Failure,
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
    amount: FloatMajorUnit,
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

impl TryFrom<&NmiRouterData<&PaymentsCompleteAuthorizeRouterData>> for NmiCompleteRequest {
    type Error = Error;
    fn try_from(
        item: &NmiRouterData<&PaymentsCompleteAuthorizeRouterData>,
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
            .change_context(ConnectorError::MissingConnectorRedirectionPayload {
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
    customer_vault_id: Option<Secret<String>>,
}

impl
    TryFrom<
        ResponseRouterData<
            CompleteAuthorize,
            NmiCompleteResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for PaymentsCompleteAuthorizeRouterData
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<
            CompleteAuthorize,
            NmiCompleteResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.transactionid),
                    redirection_data: Box::new(None),
                    mandate_reference: match item.response.customer_vault_id {
                        Some(vault_id) => Box::new(Some(
                            hyperswitch_domain_models::router_response_types::MandateReference {
                                connector_mandate_id: Some(vault_id.expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            },
                        )),
                        None => Box::new(None),
                    },
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                if item.data.request.is_auto_capture()? {
                    AttemptStatus::Charged
                } else {
                    AttemptStatus::Authorized
                },
            ),
            Response::Declined | Response::Error => (
                Err(get_nmi_error_response(item.response, item.http_code)),
                AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

fn get_nmi_error_response(response: NmiCompleteResponse, http_code: u16) -> ErrorResponse {
    ErrorResponse {
        code: response.response_code,
        message: response.responsetext.to_owned(),
        reason: Some(response.responsetext),
        status_code: http_code,
        attempt_status: None,
        connector_transaction_id: Some(response.transactionid),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
    }
}

#[derive(Debug, Serialize)]
pub struct NmiValidateRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    security_key: Secret<String>,
    #[serde(flatten)]
    payment_data: NmiValidatePaymentData,
    orderid: String,
    customer_vault: CustomerAction,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum NmiValidatePaymentData {
    ApplePay(Box<ApplePayData>),
    Card(Box<CardData>),
}

#[derive(Debug, Serialize)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    amount: FloatMajorUnit,
    security_key: Secret<String>,
    currency: Currency,
    #[serde(flatten)]
    payment_method: PaymentMethod,
    #[serde(flatten)]
    merchant_defined_field: Option<NmiMerchantDefinedField>,
    orderid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_vault: Option<CustomerAction>,
}

#[derive(Debug, Serialize)]
pub struct NmiMerchantDefinedField {
    #[serde(flatten)]
    inner: std::collections::BTreeMap<String, Secret<String>>,
}

impl NmiMerchantDefinedField {
    pub fn new(metadata: &serde_json::Value) -> Self {
        let metadata_as_string = metadata.to_string();
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
    MandatePayment(Box<MandatePayment>),
}

#[derive(Debug, Serialize)]
pub struct MandatePayment {
    customer_vault_id: Secret<String>,
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
    cavv: Option<Secret<String>>,
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

impl TryFrom<&NmiRouterData<&PaymentsAuthorizeRouterData>> for NmiPaymentsRequest {
    type Error = Error;
    fn try_from(item: &NmiRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let transaction_type = match item.router_data.request.is_auto_capture()? {
            true => TransactionType::Sale,
            false => TransactionType::Auth,
        };
        let auth_type: NmiAuthType = (&item.router_data.connector_auth_type).try_into()?;
        let amount = item.amount;

        match item
            .router_data
            .request
            .mandate_id
            .clone()
            .and_then(|mandate_ids| mandate_ids.mandate_reference_id)
        {
            Some(api_models::payments::MandateReferenceId::ConnectorMandateId(
                connector_mandate_id,
            )) => Ok(Self {
                transaction_type,
                security_key: auth_type.api_key,
                amount,
                currency: item.router_data.request.currency,
                payment_method: PaymentMethod::MandatePayment(Box::new(MandatePayment {
                    customer_vault_id: Secret::new(
                        connector_mandate_id
                            .get_connector_mandate_id()
                            .ok_or(ConnectorError::MissingConnectorMandateID)?,
                    ),
                })),
                merchant_defined_field: item
                    .router_data
                    .request
                    .metadata
                    .as_ref()
                    .map(NmiMerchantDefinedField::new),
                orderid: item.router_data.connector_request_reference_id.clone(),
                customer_vault: None,
            }),
            Some(api_models::payments::MandateReferenceId::NetworkMandateId(_))
            | Some(api_models::payments::MandateReferenceId::NetworkTokenWithNTI(_)) => {
                Err(ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("nmi"),
                ))?
            }
            None => {
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
                    customer_vault: item
                        .router_data
                        .request
                        .is_mandate_payment()
                        .then_some(CustomerAction::AddCustomer),
                })
            }
        }
    }
}

impl TryFrom<(&PaymentMethodData, Option<&PaymentsAuthorizeRouterData>)> for PaymentMethod {
    type Error = Error;
    fn try_from(
        item: (&PaymentMethodData, Option<&PaymentsAuthorizeRouterData>),
    ) -> Result<Self, Self::Error> {
        let (payment_method_data, router_data) = item;
        match payment_method_data {
            PaymentMethodData::Card(ref card) => match router_data {
                Some(data) => match data.auth_type {
                    AuthenticationType::NoThreeDs => Ok(Self::try_from(card)?),
                    AuthenticationType::ThreeDs => Ok(Self::try_from((card, &data.request))?),
                },
                None => Ok(Self::try_from(card)?),
            },
            PaymentMethodData::Wallet(ref wallet_type) => match wallet_type {
                WalletData::GooglePay(ref googlepay_data) => Ok(Self::try_from(googlepay_data)?),
                WalletData::ApplePay(ref applepay_data) => Ok(Self::try_from(applepay_data)?),
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::Paysera(_)
                | WalletData::Skrill(_)
                | WalletData::BluecodeRedirect {}
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
                | WalletData::AmazonPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_)
                | WalletData::Mifinity(_)
                | WalletData::RevolutPay(_) => Err(report!(ConnectorError::NotImplemented(
                    get_unimplemented_payment_method_error_message("nmi"),
                ))),
            },
            PaymentMethodData::CardRedirect(_)
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
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => Err(
                ConnectorError::NotImplemented(get_unimplemented_payment_method_error_message(
                    "nmi",
                ))
                .into(),
            ),
        }
    }
}

impl TryFrom<(&Card, &PaymentsAuthorizeData)> for PaymentMethod {
    type Error = Error;
    fn try_from(val: (&Card, &PaymentsAuthorizeData)) -> Result<Self, Self::Error> {
        let (card_data, item) = val;
        let auth_data = &item.get_authentication_data()?;
        let ccexp = card_data.get_card_expiry_month_year_2_digit_with_delimiter("".to_string())?;

        let card_3ds_details = CardThreeDsData {
            ccnumber: card_data.card_number.clone(),
            ccexp,
            cvv: card_data.card_cvc.clone(),
            email: item.email.clone(),
            cavv: Some(auth_data.cavv.clone()),
            eci: auth_data.eci.clone(),
            cardholder_auth: None,
            three_ds_version: auth_data
                .message_version
                .clone()
                .map(|version| version.to_string()),
            directory_server_id: auth_data
                .threeds_server_transaction_id
                .clone()
                .map(Secret::new),
        };

        Ok(Self::CardThreeDs(Box::new(card_3ds_details)))
    }
}

impl TryFrom<&Card> for PaymentMethod {
    type Error = Error;
    fn try_from(card: &Card) -> Result<Self, Self::Error> {
        let ccexp = card.get_card_expiry_month_year_2_digit_with_delimiter("".to_string())?;
        let card = CardData {
            ccnumber: card.card_number.clone(),
            ccexp,
            cvv: card.card_cvc.clone(),
        };
        Ok(Self::CardNonThreeDs(Box::new(card)))
    }
}

impl TryFrom<&GooglePayWalletData> for PaymentMethod {
    type Error = Report<ConnectorError>;
    fn try_from(wallet_data: &GooglePayWalletData) -> Result<Self, Self::Error> {
        let gpay_data = GooglePayData {
            googlepay_payment_data: Secret::new(
                wallet_data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .change_context(ConnectorError::MissingRequiredField {
                        field_name: "gpay wallet_token",
                    })?
                    .clone(),
            ),
        };
        Ok(Self::GPay(Box::new(gpay_data)))
    }
}

impl TryFrom<&ApplePayWalletData> for PaymentMethod {
    type Error = Error;
    fn try_from(apple_pay_wallet_data: &ApplePayWalletData) -> Result<Self, Self::Error> {
        let apple_pay_encrypted_data = apple_pay_wallet_data
            .payment_data
            .get_encrypted_apple_pay_payment_data_mandatory()
            .change_context(ConnectorError::MissingRequiredField {
                field_name: "Apple pay encrypted data",
            })?;

        let base64_decoded_apple_pay_data = base64::prelude::BASE64_STANDARD
            .decode(apple_pay_encrypted_data)
            .change_context(ConnectorError::InvalidDataFormat {
                field_name: "apple_pay_encrypted_data",
            })?;

        let hex_encoded_apple_pay_data = hex::encode(base64_decoded_apple_pay_data);

        let apple_pay_data = ApplePayData {
            applepay_payment_data: Secret::new(hex_encoded_apple_pay_data),
        };
        Ok(Self::ApplePay(Box::new(apple_pay_data)))
    }
}

impl TryFrom<&SetupMandateRouterData> for NmiValidateRequest {
    type Error = Error;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        match item.request.amount {
            Some(amount) if amount > 0 => Err(ConnectorError::FlowNotSupported {
                flow: "Setup Mandate with non zero amount".to_string(),
                connector: "NMI".to_string(),
            }
            .into()),
            _ => {
                if let PaymentMethodData::Card(card_details) = &item.request.payment_method_data {
                    let auth_type: NmiAuthType = (&item.connector_auth_type).try_into()?;

                    let card_data = CardData {
                        ccnumber: card_details.card_number.clone(),
                        ccexp: card_details
                            .get_card_expiry_month_year_2_digit_with_delimiter("".to_string())?,
                        cvv: card_details.card_cvc.clone(),
                    };
                    Ok(Self {
                        transaction_type: TransactionType::Validate,
                        security_key: auth_type.api_key,
                        payment_data: NmiValidatePaymentData::Card(Box::new(card_data)),
                        orderid: item.connector_request_reference_id.clone(),
                        customer_vault: CustomerAction::AddCustomer,
                    })
                } else if let PaymentMethodData::Wallet(WalletData::ApplePay(
                    apple_pay_wallet_data,
                )) = &item.request.payment_method_data
                {
                    let auth_type: NmiAuthType = (&item.connector_auth_type).try_into()?;

                    let apple_pay_encrypted_data = apple_pay_wallet_data
                        .payment_data
                        .get_encrypted_apple_pay_payment_data_mandatory()
                        .change_context(ConnectorError::MissingRequiredField {
                            field_name: "Apple pay encrypted data",
                        })?;

                    let base64_decoded_apple_pay_data = base64::prelude::BASE64_STANDARD
                        .decode(apple_pay_encrypted_data)
                        .change_context(ConnectorError::InvalidDataFormat {
                            field_name: "apple_pay_encrypted_data",
                        })?;

                    let hex_encoded_apple_pay_data = hex::encode(base64_decoded_apple_pay_data);

                    let apple_pay_data = ApplePayData {
                        applepay_payment_data: Secret::new(hex_encoded_apple_pay_data),
                    };

                    Ok(Self {
                        transaction_type: TransactionType::Validate,
                        security_key: auth_type.api_key,
                        payment_data: NmiValidatePaymentData::ApplePay(Box::new(apple_pay_data)),
                        orderid: item.connector_request_reference_id.clone(),
                        customer_vault: CustomerAction::AddCustomer,
                    })
                } else {
                    Err(ConnectorError::NotImplemented(
                        get_unimplemented_payment_method_error_message("Nmi"),
                    )
                    .into())
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NmiSyncRequest {
    pub order_id: String,
    pub security_key: Secret<String>,
}

impl TryFrom<&PaymentsSyncRouterData> for NmiSyncRequest {
    type Error = Error;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
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
    pub amount: Option<FloatMajorUnit>,
}

impl TryFrom<&NmiRouterData<&PaymentsCaptureRouterData>> for NmiCaptureRequest {
    type Error = Error;
    fn try_from(item: &NmiRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
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
        ResponseRouterData<Capture, StandardResponse, PaymentsCaptureData, PaymentsResponseData>,
    > for RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<
            Capture,
            StandardResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.transactionid.to_owned(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                AttemptStatus::Charged,
            ),
            Response::Declined | Response::Error => (
                Err(get_standard_error_response(item.response, item.http_code)),
                AttemptStatus::CaptureFailed,
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
    pub void_reason: NmiVoidReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NmiVoidReason {
    Fraud,
    UserCancel,
    IccRejected,
    IccCardRemoved,
    IccNoConfirmation,
    PosTimeout,
}

impl TryFrom<&PaymentsCancelRouterData> for NmiCancelRequest {
    type Error = Error;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        match &item.request.cancellation_reason {
            Some(cancellation_reason) => {
                let void_reason: NmiVoidReason = serde_json::from_str(&format!("\"{cancellation_reason}\"", ))
                    .map_err(|_| ConnectorError::NotSupported {
                        message: format!("Json deserialise error: unknown variant `{cancellation_reason}` expected to be one of `fraud`, `user_cancel`, `icc_rejected`,  `icc_card_removed`, `icc_no_confirmation`, `pos_timeout`. This cancellation_reason"),
                        connector: "nmi"
                    })?;
                Ok(Self {
                    transaction_type: TransactionType::Void,
                    security_key: auth.api_key,
                    transactionid: item.request.connector_transaction_id.clone(),
                    void_reason,
                })
            }
            None => Err(ConnectorError::MissingRequiredField {
                field_name: "cancellation_reason",
            }
            .into()),
        }
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
    pub customer_vault_id: Option<Secret<String>>,
}

impl<T> TryFrom<ResponseRouterData<SetupMandate, StandardResponse, T, PaymentsResponseData>>
    for RouterData<SetupMandate, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<SetupMandate, StandardResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.transactionid.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: match item.response.customer_vault_id {
                        Some(vault_id) => Box::new(Some(
                            hyperswitch_domain_models::router_response_types::MandateReference {
                                connector_mandate_id: Some(vault_id.expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            },
                        )),
                        None => Box::new(None),
                    },
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                AttemptStatus::Charged,
            ),
            Response::Declined | Response::Error => (
                Err(get_standard_error_response(item.response, item.http_code)),
                AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}
fn get_standard_error_response(response: StandardResponse, http_code: u16) -> ErrorResponse {
    ErrorResponse {
        code: response.response_code,
        message: response.responsetext.to_owned(),
        reason: Some(response.responsetext),
        status_code: http_code,
        attempt_status: None,
        connector_transaction_id: Some(response.transactionid),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
    }
}

impl TryFrom<PaymentsResponseRouterData<StandardResponse>>
    for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            StandardResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.transactionid.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: match item.response.customer_vault_id {
                        Some(vault_id) => Box::new(Some(
                            hyperswitch_domain_models::router_response_types::MandateReference {
                                connector_mandate_id: Some(vault_id.expose()),
                                payment_method_id: None,
                                mandate_metadata: None,
                                connector_mandate_request_reference_id: None,
                            },
                        )),
                        None => Box::new(None),
                    },
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                if item.data.request.is_auto_capture()? {
                    AttemptStatus::Charged
                } else {
                    AttemptStatus::Authorized
                },
            ),
            Response::Declined | Response::Error => (
                Err(get_standard_error_response(item.response, item.http_code)),
                AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<T> TryFrom<ResponseRouterData<Void, StandardResponse, T, PaymentsResponseData>>
    for RouterData<Void, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<Void, StandardResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.transactionid.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.orderid),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                AttemptStatus::VoidInitiated,
            ),
            Response::Declined | Response::Error => (
                Err(get_standard_error_response(item.response, item.http_code)),
                AttemptStatus::VoidFailed,
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

impl<F, T> TryFrom<ResponseRouterData<F, SyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, SyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.transaction {
            Some(trn) => Ok(Self {
                status: AttemptStatus::from(NmiStatus::from(trn.condition)),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(trn.transaction_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
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
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        query_response
            .parse_xml::<Self>()
            .change_context(ConnectorError::ResponseDeserializationFailed)
    }
}

impl TryFrom<Vec<u8>> for NmiRefundSyncResponse {
    type Error = Error;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let query_response = String::from_utf8(bytes)
            .change_context(ConnectorError::ResponseDeserializationFailed)?;
        query_response
            .parse_xml::<Self>()
            .change_context(ConnectorError::ResponseDeserializationFailed)
    }
}

impl From<NmiStatus> for AttemptStatus {
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
    amount: FloatMajorUnit,
}

impl<F> TryFrom<&NmiRouterData<&RefundsRouterData<F>>> for NmiRefundRequest {
    type Error = Error;
    fn try_from(item: &NmiRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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

impl TryFrom<RefundsResponseRouterData<Execute, StandardResponse>> for RefundsRouterData<Execute> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<Execute, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.orderid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Capture, StandardResponse>> for RefundsRouterData<Capture> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<Capture, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transactionid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<Response> for RefundStatus {
    fn from(item: Response) -> Self {
        match item {
            Response::Approved => Self::Success,
            Response::Declined | Response::Error => Self::Failure,
        }
    }
}

impl TryFrom<&RefundSyncRouterData> for NmiSyncRequest {
    type Error = Error;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            security_key: auth.api_key,
            order_id: item
                .request
                .connector_refund_id
                .clone()
                .ok_or(ConnectorError::MissingConnectorRefundID)?,
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, NmiRefundSyncResponse>> for RefundsRouterData<RSync> {
    type Error = Report<ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, NmiRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status =
            RefundStatus::from(NmiStatus::from(item.response.transaction.condition));
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.order_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<NmiStatus> for RefundStatus {
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

pub fn get_nmi_webhook_event(status: NmiWebhookEventType) -> IncomingWebhookEvent {
    match status {
        NmiWebhookEventType::SaleSuccess => IncomingWebhookEvent::PaymentIntentSuccess,
        NmiWebhookEventType::SaleFailure => IncomingWebhookEvent::PaymentIntentFailure,
        NmiWebhookEventType::RefundSuccess => IncomingWebhookEvent::RefundSuccess,
        NmiWebhookEventType::RefundFailure => IncomingWebhookEvent::RefundFailure,
        NmiWebhookEventType::VoidSuccess => IncomingWebhookEvent::PaymentIntentCancelled,
        NmiWebhookEventType::AuthSuccess => IncomingWebhookEvent::PaymentIntentAuthorizationSuccess,
        NmiWebhookEventType::CaptureSuccess => IncomingWebhookEvent::PaymentIntentCaptureSuccess,
        NmiWebhookEventType::AuthFailure => IncomingWebhookEvent::PaymentIntentAuthorizationFailure,
        NmiWebhookEventType::CaptureFailure => IncomingWebhookEvent::PaymentIntentCaptureFailure,
        NmiWebhookEventType::VoidFailure => IncomingWebhookEvent::PaymentIntentCancelFailure,
        NmiWebhookEventType::SaleUnknown
        | NmiWebhookEventType::RefundUnknown
        | NmiWebhookEventType::AuthUnknown
        | NmiWebhookEventType::VoidUnknown
        | NmiWebhookEventType::CaptureUnknown => IncomingWebhookEvent::EventNotSupported,
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
