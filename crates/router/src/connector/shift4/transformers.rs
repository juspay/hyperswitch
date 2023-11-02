use api_models::payments;
use cards::CardNumber;
use common_utils::pii::SecretSerdeValue;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{
        self, to_connector_meta, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, RouterData,
    },
    core::errors,
    pii, services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct Shift4PaymentsRequest {
    amount: String,
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
    CardToken(String),
}

impl<T> TryFrom<&types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for Shift4PaymentsRequest
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let submit_for_settlement = item.request.is_auto_capture()?;
        let amount = item.request.amount.to_string();
        let currency = item.request.currency;
        let payment_method = Shift4PaymentMethod::try_from(item)?;
        Ok(Self {
            amount,
            currency,
            captured: submit_for_settlement,
            payment_method,
        })
    }
}

impl<T> TryFrom<&types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for Shift4PaymentMethod
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            payments::PaymentMethodData::Card(ref ccard) => Self::try_from((item, ccard)),
            payments::PaymentMethodData::BankRedirect(ref redirect) => {
                Self::try_from((item, redirect))
            }
            payments::PaymentMethodData::Wallet(ref wallet_data) => Self::try_from(wallet_data),
            payments::PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from(bank_transfer_data.as_ref())
            }
            payments::PaymentMethodData::Voucher(ref voucher_data) => Self::try_from(voucher_data),
            payments::PaymentMethodData::GiftCard(ref giftcard_data) => {
                Self::try_from(giftcard_data.as_ref())
            }
            payments::PaymentMethodData::CardRedirect(_)
            | payments::PaymentMethodData::PayLater(_)
            | payments::PaymentMethodData::BankDebit(_)
            | payments::PaymentMethodData::Crypto(_)
            | payments::PaymentMethodData::MandatePayment
            | payments::PaymentMethodData::Reward
            | payments::PaymentMethodData::Upi(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "Shift4",
            }
            .into()),
        }
    }
}

impl TryFrom<&api_models::payments::WalletData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(wallet_data: &api_models::payments::WalletData) -> Result<Self, Self::Error> {
        match wallet_data {
            payments::WalletData::AliPayRedirect(_)
            | payments::WalletData::ApplePay(_)
            | payments::WalletData::WeChatPayRedirect(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
            payments::WalletData::AliPayQr(_)
            | payments::WalletData::AliPayHkRedirect(_)
            | payments::WalletData::MomoRedirect(_)
            | payments::WalletData::KakaoPayRedirect(_)
            | payments::WalletData::GoPayRedirect(_)
            | payments::WalletData::GcashRedirect(_)
            | payments::WalletData::ApplePayRedirect(_)
            | payments::WalletData::ApplePayThirdPartySdk(_)
            | payments::WalletData::DanaRedirect {}
            | payments::WalletData::GooglePay(_)
            | payments::WalletData::GooglePayRedirect(_)
            | payments::WalletData::GooglePayThirdPartySdk(_)
            | payments::WalletData::MbWayRedirect(_)
            | payments::WalletData::MobilePayRedirect(_)
            | payments::WalletData::PaypalRedirect(_)
            | payments::WalletData::PaypalSdk(_)
            | payments::WalletData::SamsungPay(_)
            | payments::WalletData::TwintRedirect {}
            | payments::WalletData::VippsRedirect {}
            | payments::WalletData::TouchNGoRedirect(_)
            | payments::WalletData::WeChatPayQr(_)
            | payments::WalletData::CashappQr(_)
            | payments::WalletData::SwishQr(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "Shift4",
            }
            .into()),
        }
    }
}

impl TryFrom<&api_models::payments::BankTransferData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(
        bank_transfer_data: &api_models::payments::BankTransferData,
    ) -> Result<Self, Self::Error> {
        match bank_transfer_data {
            payments::BankTransferData::MultibancoBankTransfer { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
            payments::BankTransferData::AchBankTransfer { .. }
            | payments::BankTransferData::SepaBankTransfer { .. }
            | payments::BankTransferData::BacsBankTransfer { .. }
            | payments::BankTransferData::PermataBankTransfer { .. }
            | payments::BankTransferData::BcaBankTransfer { .. }
            | payments::BankTransferData::BniVaBankTransfer { .. }
            | payments::BankTransferData::BriVaBankTransfer { .. }
            | payments::BankTransferData::CimbVaBankTransfer { .. }
            | payments::BankTransferData::DanamonVaBankTransfer { .. }
            | payments::BankTransferData::MandiriVaBankTransfer { .. }
            | payments::BankTransferData::Pix {}
            | payments::BankTransferData::Pse {} => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "Shift4",
            }
            .into()),
        }
    }
}

impl TryFrom<&api_models::payments::VoucherData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(voucher_data: &api_models::payments::VoucherData) -> Result<Self, Self::Error> {
        match voucher_data {
            payments::VoucherData::Boleto(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
            payments::VoucherData::Efecty
            | payments::VoucherData::PagoEfectivo
            | payments::VoucherData::RedCompra
            | payments::VoucherData::RedPagos
            | payments::VoucherData::Alfamart(_)
            | payments::VoucherData::Indomaret(_)
            | payments::VoucherData::Oxxo
            | payments::VoucherData::SevenEleven(_)
            | payments::VoucherData::Lawson(_)
            | payments::VoucherData::MiniStop(_)
            | payments::VoucherData::FamilyMart(_)
            | payments::VoucherData::Seicomart(_)
            | payments::VoucherData::PayEasy(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "Shift4",
            }
            .into()),
        }
    }
}

impl TryFrom<&api_models::payments::GiftCardData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(gift_card_data: &api_models::payments::GiftCardData) -> Result<Self, Self::Error> {
        match gift_card_data {
            payments::GiftCardData::Givex(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                connector: "Shift4",
            }
            .into()),
            payments::GiftCardData::PaySafeCard {} => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
        }
    }
}

impl<T>
    TryFrom<(
        &types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        &api_models::payments::Card,
    )> for Shift4PaymentMethod
{
    type Error = Error;
    fn try_from(
        (item, card): (
            &types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
            &api_models::payments::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let card_object = Card {
            number: card.card_number.clone(),
            exp_month: card.card_exp_month.clone(),
            exp_year: card.card_exp_year.clone(),
            cardholder_name: card.card_holder_name.clone(),
        };
        if item.is_three_ds() {
            Ok(Self::Cards3DSRequest(Box::new(Cards3DSRequest {
                card_number: card_object.number,
                card_exp_month: card_object.exp_month,
                card_exp_year: card_object.exp_year,
                return_url: item
                    .request
                    .complete_authorize_url
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

impl<T>
    TryFrom<(
        &types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        &payments::BankRedirectData,
    )> for Shift4PaymentMethod
{
    type Error = Error;
    fn try_from(
        (item, redirect_data): (
            &types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
            &payments::BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let flow = Flow::try_from(&item.request.router_return_url)?;
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

impl<T> TryFrom<&types::RouterData<T, types::CompleteAuthorizeData, types::PaymentsResponseData>>
    for Shift4PaymentsRequest
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, types::CompleteAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match &item.request.payment_method_data {
            Some(api::PaymentMethodData::Card(_)) => {
                let card_token: Shift4CardToken =
                    to_connector_meta(item.request.connector_meta.clone())?;
                Ok(Self {
                    amount: item.request.amount.to_string(),
                    currency: item.request.currency,
                    payment_method: Shift4PaymentMethod::CardsNon3DSRequest(Box::new(
                        CardsNon3DSRequest {
                            card: CardPayment::CardToken(card_token.id),
                            description: item.description.clone(),
                        },
                    )),
                    captured: item.request.is_auto_capture()?,
                })
            }
            Some(payments::PaymentMethodData::Wallet(_))
            | Some(payments::PaymentMethodData::GiftCard(_))
            | Some(payments::PaymentMethodData::CardRedirect(_))
            | Some(payments::PaymentMethodData::PayLater(_))
            | Some(payments::PaymentMethodData::BankDebit(_))
            | Some(payments::PaymentMethodData::BankRedirect(_))
            | Some(payments::PaymentMethodData::BankTransfer(_))
            | Some(payments::PaymentMethodData::Crypto(_))
            | Some(payments::PaymentMethodData::MandatePayment)
            | Some(payments::PaymentMethodData::Voucher(_))
            | Some(payments::PaymentMethodData::Reward)
            | Some(payments::PaymentMethodData::Upi(_))
            | None => Err(errors::ConnectorError::NotSupported {
                message: "Flow".to_string(),
                connector: "Shift4",
            }
            .into()),
        }
    }
}

impl TryFrom<&payments::BankRedirectData> for PaymentMethodType {
    type Error = Error;
    fn try_from(value: &payments::BankRedirectData) -> Result<Self, Self::Error> {
        match value {
            payments::BankRedirectData::Eps { .. } => Ok(Self::Eps),
            payments::BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            payments::BankRedirectData::Ideal { .. } => Ok(Self::Ideal),
            payments::BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            payments::BankRedirectData::BancontactCard { .. }
            | payments::BankRedirectData::Blik { .. }
            | payments::BankRedirectData::Trustly { .. }
            | payments::BankRedirectData::Przelewy24 { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
            payments::BankRedirectData::Bizum {}
            | payments::BankRedirectData::Interac { .. }
            | payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | payments::BankRedirectData::OnlineBankingFinland { .. }
            | payments::BankRedirectData::OnlineBankingPoland { .. }
            | payments::BankRedirectData::OnlineBankingSlovakia { .. }
            | payments::BankRedirectData::OpenBankingUk { .. }
            | payments::BankRedirectData::OnlineBankingFpx { .. }
            | payments::BankRedirectData::OnlineBankingThailand { .. } => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::SELECTED_PAYMENT_METHOD.to_string(),
                    connector: "Shift4",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&Option<String>> for Flow {
    type Error = Error;
    fn try_from(router_return_url: &Option<String>) -> Result<Self, Self::Error> {
        Ok(Self {
            return_url: router_return_url
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
        })
    }
}

impl<T> TryFrom<&types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for Billing
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let billing_address = item
            .address
            .billing
            .as_ref()
            .and_then(|billing| billing.address.as_ref());
        let address = get_address_details(billing_address);
        Ok(Self {
            name: billing_address.map(|billing| {
                Secret::new(format!("{:?} {:?}", billing.first_name, billing.last_name))
            }),
            email: item.request.email.clone(),
            address,
        })
    }
}

fn get_address_details(address_details: Option<&payments::AddressDetails>) -> Option<Address> {
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

impl TryFrom<&types::ConnectorAuthType> for Shift4AuthType {
    type Error = Error;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = item {
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

impl ForeignFrom<(bool, Option<&NextAction>, Shift4PaymentStatus)> for enums::AttemptStatus {
    fn foreign_from(item: (bool, Option<&NextAction>, Shift4PaymentStatus)) -> Self {
        let (captured, next_action, payment_status) = item;
        match payment_status {
            Shift4PaymentStatus::Successful => {
                if captured {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            Shift4PaymentStatus::Failed => Self::Failure,
            Shift4PaymentStatus::Pending => match next_action {
                Some(NextAction::Redirect) => Self::AuthenticationPending,
                Some(NextAction::Wait) | Some(NextAction::None) | None => Self::Pending,
            },
        }
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

#[derive(Default, Debug, Deserialize)]
pub struct Shift4NonThreeDsResponse {
    pub id: String,
    pub currency: String,
    pub amount: u32,
    pub status: Shift4PaymentStatus,
    pub captured: bool,
    pub refunded: bool,
    pub flow: Option<FlowResponse>,
}

#[derive(Default, Debug, Deserialize)]
pub struct Shift4ThreeDsResponse {
    pub enrolled: bool,
    pub version: Option<String>,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: Option<Url>,
    pub token: Token,
}

#[derive(Default, Debug, Deserialize)]
pub struct Token {
    pub id: String,
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

#[derive(Default, Debug, Deserialize)]
pub struct ThreeDSecureInfo {
    pub amount: i64,
    pub currency: String,
    pub enrolled: bool,
    #[serde(rename = "liabilityShift")]
    pub liability_shift: Option<String>,
    pub version: String,
    #[serde(rename = "authenticationFlow")]
    pub authentication_flow: Option<SecretSerdeValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowResponse {
    pub next_action: Option<NextAction>,
    pub redirect: Option<Redirect>,
    pub return_url: Option<Url>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Redirect {
    pub redirect_url: Option<Url>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NextAction {
    Redirect,
    Wait,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shift4CardToken {
    pub id: String,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            Shift4ThreeDsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            Shift4ThreeDsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .redirect_url
            .map(|url| services::RedirectForm::from((url, services::Method::Get)));
        Ok(Self {
            status: if redirection_data.is_some() {
                enums::AttemptStatus::AuthenticationPending
            } else {
                enums::AttemptStatus::Pending
            },
            request: types::PaymentsAuthorizeData {
                enrolled_for_3ds: item.response.enrolled,
                ..item.data.request
            },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data,
                mandate_reference: None,
                connector_metadata: Some(
                    serde_json::to_value(Shift4CardToken {
                        id: item.response.token.id,
                    })
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
                ),
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

impl<T, F>
    TryFrom<types::ResponseRouterData<F, Shift4NonThreeDsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            Shift4NonThreeDsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_id = types::ResponseId::ConnectorTransactionId(item.response.id.clone());
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.captured,
                item.response
                    .flow
                    .as_ref()
                    .and_then(|flow| flow.next_action.as_ref()),
                item.response.status,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: connector_id,
                redirection_data: item
                    .response
                    .flow
                    .and_then(|flow| flow.redirect)
                    .and_then(|redirect| redirect.redirect_url)
                    .map(|url| services::RedirectForm::from((url, services::Method::Get))),
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
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
    amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for Shift4RefundRequest {
    type Error = Error;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            charge_id: item.request.connector_transaction_id.clone(),
            amount: item.request.refund_amount,
        })
    }
}

impl From<Shift4RefundStatus> for enums::RefundStatus {
    fn from(item: Shift4RefundStatus) -> Self {
        match item {
            self::Shift4RefundStatus::Successful => Self::Success,
            self::Shift4RefundStatus::Failed => Self::Failure,
            self::Shift4RefundStatus::Processing => Self::Pending,
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

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = Error;
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = Error;
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

#[derive(Debug, Default, Deserialize)]
pub struct ErrorResponse {
    pub error: ApiErrorResponse,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
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

impl From<Shift4WebhookEvent> for api::IncomingWebhookEvent {
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
