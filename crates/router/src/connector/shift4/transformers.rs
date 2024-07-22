use api_models::payments;
use cards::CardNumber;
use common_utils::pii::SecretSerdeValue;
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{
        self, to_connector_meta, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, PaymentsPreProcessingData, RouterData,
    },
    core::errors,
    pii, services,
    types::{self, api, domain, storage::enums, transformers::ForeignFrom},
};

type Error = error_stack::Report<errors::ConnectorError>;

trait Shift4AuthorizePreprocessingCommon {
    fn is_automatic_capture(&self) -> Result<bool, Error>;
    fn get_router_return_url(&self) -> Option<String>;
    fn get_email_optional(&self) -> Option<pii::Email>;
    fn get_complete_authorize_url(&self) -> Option<String>;
    fn get_amount_required(&self) -> Result<i64, Error>;
    fn get_currency_required(&self) -> Result<diesel_models::enums::Currency, Error>;
    fn get_payment_method_data_required(&self) -> Result<domain::PaymentMethodData, Error>;
}

impl Shift4AuthorizePreprocessingCommon for types::PaymentsAuthorizeData {
    fn get_email_optional(&self) -> Option<pii::Email> {
        self.email.clone()
    }

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_amount_required(&self) -> Result<i64, error_stack::Report<errors::ConnectorError>> {
        Ok(self.amount)
    }

    fn get_currency_required(
        &self,
    ) -> Result<diesel_models::enums::Currency, error_stack::Report<errors::ConnectorError>> {
        Ok(self.currency)
    }
    fn get_payment_method_data_required(
        &self,
    ) -> Result<domain::PaymentMethodData, error_stack::Report<errors::ConnectorError>> {
        Ok(self.payment_method_data.clone())
    }

    fn is_automatic_capture(&self) -> Result<bool, Error> {
        self.is_auto_capture()
    }

    fn get_router_return_url(&self) -> Option<String> {
        self.router_return_url.clone()
    }
}

impl Shift4AuthorizePreprocessingCommon for types::PaymentsPreProcessingData {
    fn get_email_optional(&self) -> Option<pii::Email> {
        self.email.clone()
    }

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_amount_required(&self) -> Result<i64, Error> {
        self.get_amount()
    }

    fn get_currency_required(&self) -> Result<diesel_models::enums::Currency, Error> {
        self.get_currency()
    }
    fn get_payment_method_data_required(&self) -> Result<domain::PaymentMethodData, Error> {
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
    CardToken(Secret<String>),
}

impl<T, Req> TryFrom<&types::RouterData<T, Req, types::PaymentsResponseData>>
    for Shift4PaymentsRequest
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, Req, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let submit_for_settlement = item.request.is_automatic_capture()?;
        let amount = item.request.get_amount_required()?.to_string();
        let currency = item.request.get_currency_required()?;
        let payment_method = Shift4PaymentMethod::try_from(item)?;
        Ok(Self {
            amount,
            currency,
            captured: submit_for_settlement,
            payment_method,
        })
    }
}

impl<T, Req> TryFrom<&types::RouterData<T, Req, types::PaymentsResponseData>>
    for Shift4PaymentMethod
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, Req, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.request.get_payment_method_data_required()? {
            domain::PaymentMethodData::Card(ref ccard) => Self::try_from((item, ccard)),
            domain::PaymentMethodData::BankRedirect(ref redirect) => {
                Self::try_from((item, redirect))
            }
            domain::PaymentMethodData::Wallet(ref wallet_data) => Self::try_from(wallet_data),
            domain::PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from(bank_transfer_data.as_ref())
            }
            domain::PaymentMethodData::Voucher(ref voucher_data) => Self::try_from(voucher_data),
            domain::PaymentMethodData::GiftCard(ref giftcard_data) => {
                Self::try_from(giftcard_data.as_ref())
            }
            domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::RealTimePayment(_)
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::OpenBanking(_)
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::WalletData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(wallet_data: &domain::WalletData) -> Result<Self, Self::Error> {
        match wallet_data {
            domain::WalletData::AliPayRedirect(_)
            | domain::WalletData::ApplePay(_)
            | domain::WalletData::WeChatPayRedirect(_)
            | domain::WalletData::AliPayQr(_)
            | domain::WalletData::AliPayHkRedirect(_)
            | domain::WalletData::MomoRedirect(_)
            | domain::WalletData::KakaoPayRedirect(_)
            | domain::WalletData::GoPayRedirect(_)
            | domain::WalletData::GcashRedirect(_)
            | domain::WalletData::ApplePayRedirect(_)
            | domain::WalletData::ApplePayThirdPartySdk(_)
            | domain::WalletData::DanaRedirect {}
            | domain::WalletData::GooglePay(_)
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
            | domain::WalletData::WeChatPayQr(_)
            | domain::WalletData::CashappQr(_)
            | domain::WalletData::SwishQr(_)
            | domain::WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
        }
    }
}

impl TryFrom<&domain::BankTransferData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(bank_transfer_data: &domain::BankTransferData) -> Result<Self, Self::Error> {
        match bank_transfer_data {
            domain::BankTransferData::MultibancoBankTransfer { .. }
            | domain::BankTransferData::AchBankTransfer { .. }
            | domain::BankTransferData::SepaBankTransfer { .. }
            | domain::BankTransferData::BacsBankTransfer { .. }
            | domain::BankTransferData::PermataBankTransfer { .. }
            | domain::BankTransferData::BcaBankTransfer { .. }
            | domain::BankTransferData::BniVaBankTransfer { .. }
            | domain::BankTransferData::BriVaBankTransfer { .. }
            | domain::BankTransferData::CimbVaBankTransfer { .. }
            | domain::BankTransferData::DanamonVaBankTransfer { .. }
            | domain::BankTransferData::MandiriVaBankTransfer { .. }
            | domain::BankTransferData::Pix { .. }
            | domain::BankTransferData::Pse {}
            | domain::BankTransferData::LocalBankTransfer { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&domain::VoucherData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(voucher_data: &domain::VoucherData) -> Result<Self, Self::Error> {
        match voucher_data {
            domain::VoucherData::Boleto(_)
            | domain::VoucherData::Efecty
            | domain::VoucherData::PagoEfectivo
            | domain::VoucherData::RedCompra
            | domain::VoucherData::RedPagos
            | domain::VoucherData::Alfamart(_)
            | domain::VoucherData::Indomaret(_)
            | domain::VoucherData::Oxxo
            | domain::VoucherData::SevenEleven(_)
            | domain::VoucherData::Lawson(_)
            | domain::VoucherData::MiniStop(_)
            | domain::VoucherData::FamilyMart(_)
            | domain::VoucherData::Seicomart(_)
            | domain::VoucherData::PayEasy(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
        }
    }
}

impl TryFrom<&domain::GiftCardData> for Shift4PaymentMethod {
    type Error = Error;
    fn try_from(gift_card_data: &domain::GiftCardData) -> Result<Self, Self::Error> {
        match gift_card_data {
            domain::GiftCardData::Givex(_) | domain::GiftCardData::PaySafeCard {} => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Shift4"),
                )
                .into())
            }
        }
    }
}

impl<T, Req>
    TryFrom<(
        &types::RouterData<T, Req, types::PaymentsResponseData>,
        &domain::Card,
    )> for Shift4PaymentMethod
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        (item, card): (
            &types::RouterData<T, Req, types::PaymentsResponseData>,
            &domain::Card,
        ),
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

impl<T, Req>
    TryFrom<(
        &types::RouterData<T, Req, types::PaymentsResponseData>,
        &domain::BankRedirectData,
    )> for Shift4PaymentMethod
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        (item, redirect_data): (
            &types::RouterData<T, Req, types::PaymentsResponseData>,
            &domain::BankRedirectData,
        ),
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

impl<T> TryFrom<&types::RouterData<T, types::CompleteAuthorizeData, types::PaymentsResponseData>>
    for Shift4PaymentsRequest
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, types::CompleteAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match &item.request.payment_method_data {
            Some(domain::PaymentMethodData::Card(_)) => {
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
            Some(domain::PaymentMethodData::Wallet(_))
            | Some(domain::PaymentMethodData::GiftCard(_))
            | Some(domain::PaymentMethodData::CardRedirect(_))
            | Some(domain::PaymentMethodData::PayLater(_))
            | Some(domain::PaymentMethodData::BankDebit(_))
            | Some(domain::PaymentMethodData::BankRedirect(_))
            | Some(domain::PaymentMethodData::BankTransfer(_))
            | Some(domain::PaymentMethodData::Crypto(_))
            | Some(domain::PaymentMethodData::MandatePayment)
            | Some(domain::PaymentMethodData::Voucher(_))
            | Some(domain::PaymentMethodData::Reward)
            | Some(domain::PaymentMethodData::RealTimePayment(_))
            | Some(domain::PaymentMethodData::Upi(_))
            | Some(domain::PaymentMethodData::OpenBanking(_))
            | Some(domain::PaymentMethodData::CardToken(_))
            | None => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Shift4"),
            )
            .into()),
        }
    }
}

impl TryFrom<&domain::BankRedirectData> for PaymentMethodType {
    type Error = Error;
    fn try_from(value: &domain::BankRedirectData) -> Result<Self, Self::Error> {
        match value {
            domain::BankRedirectData::Eps { .. } => Ok(Self::Eps),
            domain::BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            domain::BankRedirectData::Ideal { .. } => Ok(Self::Ideal),
            domain::BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            domain::BankRedirectData::BancontactCard { .. }
            | domain::BankRedirectData::Blik { .. }
            | domain::BankRedirectData::Trustly { .. }
            | domain::BankRedirectData::Przelewy24 { .. }
            | domain::BankRedirectData::Bizum {}
            | domain::BankRedirectData::Interac { .. }
            | domain::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | domain::BankRedirectData::OnlineBankingFinland { .. }
            | domain::BankRedirectData::OnlineBankingPoland { .. }
            | domain::BankRedirectData::OnlineBankingSlovakia { .. }
            | domain::BankRedirectData::OpenBankingUk { .. }
            | domain::BankRedirectData::OnlineBankingFpx { .. }
            | domain::BankRedirectData::OnlineBankingThailand { .. }
            | domain::BankRedirectData::LocalBankRedirect {} => {
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

impl<T, Req> TryFrom<&types::RouterData<T, Req, types::PaymentsResponseData>> for Billing
where
    Req: Shift4AuthorizePreprocessingCommon,
{
    type Error = Error;
    fn try_from(
        item: &types::RouterData<T, Req, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
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
    pub amount: i64,
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

impl TryFrom<types::PaymentsPreprocessingResponseRouterData<Shift4ThreeDsResponse>>
    for types::PaymentsPreProcessingRouterData
{
    type Error = Error;
    fn try_from(
        item: types::PaymentsPreprocessingResponseRouterData<Shift4ThreeDsResponse>,
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
            request: types::PaymentsPreProcessingData {
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
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
                ),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
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
                incremental_authorization_allowed: None,
                charge_id: None,
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
