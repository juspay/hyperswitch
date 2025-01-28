use api_models::webhooks::IncomingWebhookEvent;
use cards::CardNumber;
use common_enums::enums;
use common_utils::{
    pii::{self, SecretSerdeValue},
    request::Method,
    types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{
        BankRedirectData, BankTransferData, Card as CardData, GiftCardData, PaymentMethodData,
        VoucherData, WalletData,
    },
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsPreProcessingData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsPreProcessingRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{
        PaymentsPreprocessingResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        self, to_connector_meta, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, PaymentsPreProcessingRequestData, RouterData as _,
    },
};

type Error = error_stack::Report<errors::ConnectorError>;

trait Shift4AuthorizePreprocessingCommon {
    fn is_automatic_capture(&self) -> Result<bool, Error>;
    fn get_router_return_url(&self) -> Option<String>;
    fn get_email_optional(&self) -> Option<pii::Email>;
    fn get_complete_authorize_url(&self) -> Option<String>;
    fn get_currency_required(&self) -> Result<enums::Currency, Error>;
    fn get_payment_method_data_required(&self) -> Result<PaymentMethodData, Error>;
}

pub struct Shift4RouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(MinorUnit, T)> for Shift4RouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

impl Shift4AuthorizePreprocessingCommon for PaymentsAuthorizeData {
    fn get_email_optional(&self) -> Option<pii::Email> {
        self.email.clone()
    }

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_currency_required(
        &self,
    ) -> Result<enums::Currency, error_stack::Report<errors::ConnectorError>> {
        Ok(self.currency)
    }
    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>> {
        Ok(self.payment_method_data.clone())
    }

    fn is_automatic_capture(&self) -> Result<bool, Error> {
        self.is_auto_capture()
    }

    fn get_router_return_url(&self) -> Option<String> {
        self.router_return_url.clone()
    }
}

impl Shift4AuthorizePreprocessingCommon for PaymentsPreProcessingData {
    fn get_email_optional(&self) -> Option<pii::Email> {
        self.email.clone()
    }

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_currency_required(&self) -> Result<enums::Currency, Error> {
        self.get_currency()
    }
    fn get_payment_method_data_required(&self) -> Result<PaymentMethodData, Error> {
        self.payment_method_data.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_data",
            }
            .into(),
        )
    }
    fn is_automatic_capture(&self) -> Result<bool, Error> {
        self.is_auto_capture()
    }

    fn get_router_return_url(&self) -> Option<String> {
        self.router_return_url.clone()
    }
}
#[derive(Debug, Serialize)]
pub struct Shift4PaymentsRequest {
    amount: MinorUnit,
    currency: enums::Currency,
    captured: bool,
    #[serde(flatten)]
    payment_method: Shift4PaymentMethod,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Shift4PaymentMethod {
    CardsNon3DSRequest(Box<CardsNon3DSRequest>),
    BankRedirectRequest(Box<BankRedirectRequest>),
    Cards3DSRequest(Box<Cards3DSRequest>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectRequest {
    payment_method: PaymentMethod,
    flow: Flow,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cards3DSRequest {
    #[serde(rename = "card[number]")]
    pub card_number: CardNumber,
    #[serde(rename = "card[expMonth]")]
    pub card_exp_month: Secret<String>,
    #[serde(rename = "card[expYear]")]
    pub card_exp_year: Secret<String>,
    return_url: String,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardsNon3DSRequest {
    card: CardPayment,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    pub return_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodType {
    Eps,
    Giropay,
    Ideal,
    Sofort,
}

#[derive(Debug, Serialize)]
pub struct PaymentMethod {
    #[serde(rename = "type")]
    method_type: PaymentMethodType,
    billing: Billing,
}

#[derive(Debug, Serialize)]
pub struct Billing {
    name: Option<Secret<String>>,
    email: Option<pii::Email>,
    address: Option<Address>,
}

#[derive(Debug, Serialize)]
pub struct Address {
    line1: Option<Secret<String>>,
    line2: Option<Secret<String>>,
    zip: Option<Secret<String>>,
    state: Option<Secret<String>>,
    city: Option<String>,
    country: Option<api_models::enums::CountryAlpha2>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DeviceData;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub number: CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    pub cardholder_name: Secret<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum CardPayment {
    RawCard(Box<Card>),
    CardToken(Secret<String>),
}

impl<T, Req> TryFrom<&Shift4RouterData<&RouterData<T, Req, PaymentsResponseData>>>
    for Shift4PaymentsRequest
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        item: &Shift4RouterData<&RouterData<T, Req, PaymentsResponseData>>,
    ) -> Result<Self, Self::Error> {
        let submit_for_settlement = item.router_data.request.is_automatic_capture()?;
        let amount = item.amount.to_owned();
        let currency = item.router_data.request.get_currency_required()?;
        let payment_method = Shift4PaymentMethod::try_from(item.router_data)?;
        Ok(Self {
            amount,
            currency,
            captured: submit_for_settlement,
            payment_method,
        })
    }
}

impl<T, Req> TryFrom<&RouterData<T, Req, PaymentsResponseData>> for Shift4PaymentMethod
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(item: &RouterData<T, Req, PaymentsResponseData>) -> Result<Self, Self::Error> {
        match item.request.get_payment_method_data_required()? {
            PaymentMethodData::Card(ref ccard) => Self::try_from((item, ccard)),
            PaymentMethodData::BankRedirect(ref redirect) => Self::try_from((item, redirect)),
            PaymentMethodData::Wallet(ref wallet_data) => Self::try_from(wallet_data),
            PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from(bank_transfer_data.as_ref())
            }
            PaymentMethodData::Voucher(ref voucher_data) => Self::try_from(voucher_data),
            PaymentMethodData::GiftCard(ref giftcard_data) => {
                Self::try_from(giftcard_data.as_ref())
            }
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&WalletData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(wallet_data: &WalletData) -> Result<Self, Self::Error> {
        match wallet_data {
            WalletData::AliPayRedirect(_)
            | WalletData::AmazonPay(_)
            | WalletData::AmazonPayRedirect(_)
            | WalletData::ApplePay(_)
            | WalletData::WeChatPayRedirect(_)
            | WalletData::AliPayQr(_)
            | WalletData::AliPayHkRedirect(_)
            | WalletData::MomoRedirect(_)
            | WalletData::KakaoPayRedirect(_)
            | WalletData::GoPayRedirect(_)
            | WalletData::GcashRedirect(_)
            | WalletData::ApplePayRedirect(_)
            | WalletData::ApplePayThirdPartySdk(_)
            | WalletData::DanaRedirect {}
            | WalletData::GooglePay(_)
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
            | WalletData::WeChatPayQr(_)
            | WalletData::CashappQr(_)
            | WalletData::SwishQr(_)
            | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
        }
    }
}

impl TryFrom<&BankTransferData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(bank_transfer_data: &BankTransferData) -> Result<Self, Self::Error> {
        match bank_transfer_data {
            BankTransferData::MultibancoBankTransfer { .. }
            | BankTransferData::AchBankTransfer { .. }
            | BankTransferData::SepaBankTransfer { .. }
            | BankTransferData::BacsBankTransfer { .. }
            | BankTransferData::PermataBankTransfer { .. }
            | BankTransferData::BcaBankTransfer { .. }
            | BankTransferData::BniVaBankTransfer { .. }
            | BankTransferData::BriVaBankTransfer { .. }
            | BankTransferData::CimbVaBankTransfer { .. }
            | BankTransferData::DanamonVaBankTransfer { .. }
            | BankTransferData::MandiriVaBankTransfer { .. }
            | BankTransferData::Pix { .. }
            | BankTransferData::Pse {}
            | BankTransferData::LocalBankTransfer { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&VoucherData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(voucher_data: &VoucherData) -> Result<Self, Self::Error> {
        match voucher_data {
            VoucherData::Boleto(_)
            | VoucherData::Efecty
            | VoucherData::PagoEfectivo
            | VoucherData::RedCompra
            | VoucherData::RedPagos
            | VoucherData::Alfamart(_)
            | VoucherData::Indomaret(_)
            | VoucherData::Oxxo
            | VoucherData::SevenEleven(_)
            | VoucherData::Lawson(_)
            | VoucherData::MiniStop(_)
            | VoucherData::FamilyMart(_)
            | VoucherData::Seicomart(_)
            | VoucherData::PayEasy(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
        }
    }
}

impl TryFrom<&GiftCardData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(gift_card_data: &GiftCardData) -> Result<Self, Self::Error> {
        match gift_card_data {
            GiftCardData::Givex(_) | GiftCardData::PaySafeCard {} => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
        }
    }
}

impl<T, Req> TryFrom<(&RouterData<T, Req, PaymentsResponseData>, &CardData)> for Shift4PaymentMethod
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        (item, card): (&RouterData<T, Req, PaymentsResponseData>, &CardData),
    ) -> Result<Self, Self::Error> {
        let card_object = Card {
            number: card.card_number.clone(),
            exp_month: card.card_exp_month.clone(),
            exp_year: card.card_exp_year.clone(),
            cardholder_name: item
                .get_optional_billing_full_name()
                .unwrap_or(Secret::new("".to_string())),
        };
        if item.is_three_ds() {
            Ok(Self::Cards3DSRequest(Box::new(Cards3DSRequest {
                card_number: card_object.number,
                card_exp_month: card_object.exp_month,
                card_exp_year: card_object.exp_year,
                return_url: item
                    .request
                    .get_complete_authorize_url()
                    .clone()
                    .ok_or_else(|| errors::ConnectorError::RequestEncodingFailed)?,
            })))
        } else {
            Ok(Self::CardsNon3DSRequest(Box::new(CardsNon3DSRequest {
                card: CardPayment::RawCard(Box::new(card_object)),
                description: item.description.clone(),
            })))
        }
    }
}

impl<T, Req> TryFrom<(&RouterData<T, Req, PaymentsResponseData>, &BankRedirectData)>
    for Shift4PaymentMethod
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        (item, redirect_data): (&RouterData<T, Req, PaymentsResponseData>, &BankRedirectData),
    ) -> Result<Self, Self::Error> {
        let flow = Flow::try_from(item.request.get_router_return_url())?;
        let method_type = PaymentMethodType::try_from(redirect_data)?;
        let billing = Billing::try_from(item)?;
        let payment_method = PaymentMethod {
            method_type,
            billing,
        };
        Ok(Self::BankRedirectRequest(Box::new(BankRedirectRequest {
            payment_method,
            flow,
        })))
    }
}

impl<T> TryFrom<&Shift4RouterData<&RouterData<T, CompleteAuthorizeData, PaymentsResponseData>>>
    for Shift4PaymentsRequest
{
    type Error = Error;
    fn try_from(
        item: &Shift4RouterData<&RouterData<T, CompleteAuthorizeData, PaymentsResponseData>>,
    ) -> Result<Self, Self::Error> {
        match &item.router_data.request.payment_method_data {
            Some(PaymentMethodData::Card(_)) => {
                let card_token: Shift4CardToken =
                    to_connector_meta(item.router_data.request.connector_meta.clone())?;
                Ok(Self {
                    amount: item.amount.to_owned(),
                    currency: item.router_data.request.currency,
                    payment_method: Shift4PaymentMethod::CardsNon3DSRequest(Box::new(
                        CardsNon3DSRequest {
                            card: CardPayment::CardToken(card_token.id),
                            description: item.router_data.description.clone(),
                        },
                    )),
                    captured: item.router_data.request.is_auto_capture()?,
                })
            }
            Some(PaymentMethodData::Wallet(_))
            | Some(PaymentMethodData::GiftCard(_))
            | Some(PaymentMethodData::CardRedirect(_))
            | Some(PaymentMethodData::PayLater(_))
            | Some(PaymentMethodData::BankDebit(_))
            | Some(PaymentMethodData::BankRedirect(_))
            | Some(PaymentMethodData::BankTransfer(_))
            | Some(PaymentMethodData::Crypto(_))
            | Some(PaymentMethodData::MandatePayment)
            | Some(PaymentMethodData::Voucher(_))
            | Some(PaymentMethodData::Reward)
            | Some(PaymentMethodData::RealTimePayment(_))
            | Some(PaymentMethodData::MobilePayment(_))
            | Some(PaymentMethodData::Upi(_))
            | Some(PaymentMethodData::OpenBanking(_))
            | Some(PaymentMethodData::CardToken(_))
            | Some(PaymentMethodData::NetworkToken(_))
            | Some(PaymentMethodData::CardDetailsForNetworkTransactionId(_))
            | None => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
        }
    }
}

impl TryFrom<&BankRedirectData> for PaymentMethodType {
    type Error = Error;
    fn try_from(value: &BankRedirectData) -> Result<Self, Self::Error> {
        match value {
            BankRedirectData::Eps { .. } => Ok(Self::Eps),
            BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            BankRedirectData::Ideal { .. } => Ok(Self::Ideal),
            BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            BankRedirectData::BancontactCard { .. }
            | BankRedirectData::Blik { .. }
            | BankRedirectData::Trustly { .. }
            | BankRedirectData::Przelewy24 { .. }
            | BankRedirectData::Bizum {}
            | BankRedirectData::Interac { .. }
            | BankRedirectData::OnlineBankingCzechRepublic { .. }
            | BankRedirectData::OnlineBankingFinland { .. }
            | BankRedirectData::OnlineBankingPoland { .. }
            | BankRedirectData::OnlineBankingSlovakia { .. }
            | BankRedirectData::OpenBankingUk { .. }
            | BankRedirectData::OnlineBankingFpx { .. }
            | BankRedirectData::OnlineBankingThailand { .. }
            | BankRedirectData::LocalBankRedirect {} => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<Option<String>> for Flow {
    type Error = Error;
    fn try_from(router_return_url: Option<String>) -> Result<Self, Self::Error> {
        Ok(Self {
            return_url: router_return_url.ok_or(errors::ConnectorError::RequestEncodingFailed)?,
        })
    }
}

impl<T, Req> TryFrom<&RouterData<T, Req, PaymentsResponseData>> for Billing
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(item: &RouterData<T, Req, PaymentsResponseData>) -> Result<Self, Self::Error> {
        let billing_address = item
            .get_optional_billing()
            .as_ref()
            .and_then(|billing| billing.address.as_ref());
        let address = get_address_details(billing_address);
        Ok(Self {
            name: billing_address.map(|billing| {
                Secret::new(format!("{:?} {:?}", billing.first_name, billing.last_name))
            }),
            email: item.request.get_email_optional(),
            address,
        })
    }
}

fn get_address_details(
    address_details: Option<&hyperswitch_domain_models::address::AddressDetails>,
) -> Option<Address> {
    address_details.map(|address| Address {
        line1: address.line1.clone(),
        line2: address.line1.clone(),
        zip: address.zip.clone(),
        state: address.state.clone(),
        city: address.city.clone(),
        country: address.country,
    })
}

// Auth Struct
pub struct Shift4AuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for Shift4AuthType {
    type Error = Error;
    fn try_from(item: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::HeaderKey { api_key } = item {
            Ok(Self {
                api_key: api_key.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shift4PaymentStatus {
    Successful,
    Failed,
    #[default]
    Pending,
}

fn get_status(
    captured: bool,
    next_action: Option<&NextAction>,
    payment_status: Shift4PaymentStatus,
) -> enums::AttemptStatus {
    match payment_status {
        Shift4PaymentStatus::Successful => {
            if captured {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        Shift4PaymentStatus::Failed => enums::AttemptStatus::Failure,
        Shift4PaymentStatus::Pending => match next_action {
            Some(NextAction::Redirect) => enums::AttemptStatus::AuthenticationPending,
            Some(NextAction::Wait) | Some(NextAction::None) | None => enums::AttemptStatus::Pending,
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectEventType {
    #[serde(rename = "type")]
    pub event_type: Shift4WebhookEvent,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Shift4WebhookEvent {
    ChargeSucceeded,
    ChargeFailed,
    ChargeUpdated,
    ChargeCaptured,
    ChargeRefunded,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectData {
    pub id: String,
    pub refunds: Option<Vec<RefundIdObject>>,
}

#[derive(Debug, Deserialize)]
pub struct RefundIdObject {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectId {
    #[serde(rename = "type")]
    pub event_type: Shift4WebhookEvent,
    pub data: Shift4WebhookObjectData,
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectResource {
    pub data: serde_json::Value,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Shift4NonThreeDsResponse {
    pub id: String,
    pub currency: String,
    pub amount: u32,
    pub status: Shift4PaymentStatus,
    pub captured: bool,
    pub refunded: bool,
    pub flow: Option<FlowResponse>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Shift4ThreeDsResponse {
    pub enrolled: bool,
    pub version: Option<String>,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: Option<Url>,
    pub token: Token,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Token {
    pub id: Secret<String>,
    pub created: i64,
    #[serde(rename = "objectType")]
    pub object_type: String,
    pub first6: String,
    pub last4: String,
    pub fingerprint: Secret<String>,
    pub brand: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub country: String,
    pub used: bool,
    #[serde(rename = "threeDSecureInfo")]
    pub three_d_secure_info: ThreeDSecureInfo,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct ThreeDSecureInfo {
    pub amount: MinorUnit,
    pub currency: String,
    pub enrolled: bool,
    #[serde(rename = "liabilityShift")]
    pub liability_shift: Option<String>,
    pub version: String,
    #[serde(rename = "authenticationFlow")]
    pub authentication_flow: Option<SecretSerdeValue>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowResponse {
    pub next_action: Option<NextAction>,
    pub redirect: Option<Redirect>,
    pub return_url: Option<Url>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Redirect {
    pub redirect_url: Option<Url>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NextAction {
    Redirect,
    Wait,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shift4CardToken {
    pub id: Secret<String>,
}

impl TryFrom<PaymentsPreprocessingResponseRouterData<Shift4ThreeDsResponse>>
    for PaymentsPreProcessingRouterData
{
    type Error = Error;
    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<Shift4ThreeDsResponse>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .redirect_url
            .map(|url| RedirectForm::from((url, Method::Get)));
        Ok(Self {
            status: if redirection_data.is_some() {
                enums::AttemptStatus::AuthenticationPending
            } else {
                enums::AttemptStatus::Pending
            },
            request: PaymentsPreProcessingData {
                enrolled_for_3ds: item.response.enrolled,
                ..item.data.request
            },
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: Some(
                    serde_json::to_value(Shift4CardToken {
                        id: item.response.token.id,
                    })
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
                ),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<T, F> TryFrom<ResponseRouterData<F, Shift4NonThreeDsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, Shift4NonThreeDsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_id = ResponseId::ConnectorTransactionId(item.response.id.clone());
        Ok(Self {
            status: get_status(
                item.response.captured,
                item.response
                    .flow
                    .as_ref()
                    .and_then(|flow| flow.next_action.as_ref()),
                item.response.status,
            ),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: connector_id,
                redirection_data: Box::new(
                    item.response
                        .flow
                        .and_then(|flow| flow.redirect)
                        .and_then(|redirect| redirect.redirect_url)
                        .map(|url| RedirectForm::from((url, Method::Get))),
                ),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shift4RefundRequest {
    charge_id: String,
    amount: MinorUnit,
}

impl<F> TryFrom<&Shift4RouterData<&RefundsRouterData<F>>> for Shift4RefundRequest {
    type Error = Error;
    fn try_from(item: &Shift4RouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            charge_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount.to_owned(),
        })
    }
}

impl From<Shift4RefundStatus> for enums::RefundStatus {
    fn from(item: Shift4RefundStatus) -> Self {
        match item {
            Shift4RefundStatus::Successful => Self::Success,
            Shift4RefundStatus::Failed => Self::Failure,
            Shift4RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub charge: String,
    pub status: Shift4RefundStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shift4RefundStatus {
    Successful,
    Processing,
    #[default]
    Failed,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub error: ApiErrorResponse,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApiErrorResponse {
    pub code: Option<String>,
    pub message: String,
}

pub fn is_transaction_event(event: &Shift4WebhookEvent) -> bool {
    matches!(
        event,
        Shift4WebhookEvent::ChargeCaptured
            | Shift4WebhookEvent::ChargeFailed
            | Shift4WebhookEvent::ChargeSucceeded
            | Shift4WebhookEvent::ChargeUpdated
    )
}

pub fn is_refund_event(event: &Shift4WebhookEvent) -> bool {
    matches!(event, Shift4WebhookEvent::ChargeRefunded)
}

impl From<Shift4WebhookEvent> for IncomingWebhookEvent {
    fn from(event: Shift4WebhookEvent) -> Self {
        match event {
            Shift4WebhookEvent::ChargeSucceeded | Shift4WebhookEvent::ChargeUpdated => {
                //reference : https://dev.shift4.com/docs/api#event-types
                Self::PaymentIntentProcessing
            }
            Shift4WebhookEvent::ChargeCaptured => Self::PaymentIntentSuccess,
            Shift4WebhookEvent::ChargeFailed => Self::PaymentIntentFailure,
            Shift4WebhookEvent::ChargeRefunded => Self::RefundSuccess,
            Shift4WebhookEvent::Unknown => Self::EventNotSupported,
        }
    }
}
