use api_models::payments;
use base64::Engine;
use common_utils::pii;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, ApplePayDecrypt, CardData, PaymentsAuthorizeRequestData,
        PaymentsSetupMandateRequestData, PaymentsSyncRequestData, RouterData,
    },
    consts,
    core::errors,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums,
        transformers::ForeignFrom,
        ApplePayPredecryptData,
    },
};

#[derive(Debug, Serialize)]
pub struct CybersourceRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for CybersourceRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceZeroMandateRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
}

impl TryFrom<&types::SetupMandateRouterData> for CybersourceZeroMandateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SetupMandateRouterData) -> Result<Self, Self::Error> {
        let email = item.request.get_email()?;
        let bill_to = build_bill_to(item.get_billing()?, email)?;

        let order_information = OrderInformationWithBill {
            amount_details: Amount {
                total_amount: "0".to_string(),
                currency: item.request.currency,
            },
            bill_to: Some(bill_to),
        };
        let (action_list, action_token_types, authorization_options) = (
            Some(vec![CybersourceActionsList::TokenCreate]),
            Some(vec![CybersourceActionsTokenType::InstrumentIdentifier]),
            Some(CybersourceAuthorizationOptions {
                initiator: CybersourcePaymentInitiator {
                    initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                    credential_stored_on_file: Some(true),
                    stored_credential_used: None,
                },
                merchant_intitiated_transaction: None,
            }),
        );

        let processing_information = ProcessingInformation {
            capture: Some(false),
            capture_options: None,
            action_list,
            action_token_types,
            authorization_options,
            commerce_indicator: CybersourceCommerceIndicator::Internet,
            payment_solution: None,
        };

        let client_reference_information = ClientReferenceInformation {
            code: Some(item.connector_request_reference_id.clone()),
        };

        let payment_information = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                let card = CardDetails::PaymentCard(Card {
                    number: ccard.card_number,
                    expiration_month: ccard.card_exp_month,
                    expiration_year: ccard.card_exp_year,
                    security_code: ccard.card_cvc,
                    card_type: None,
                });
                PaymentInformation::Cards(CardPaymentInformation {
                    card,
                    instrument_identifier: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Cybersource"),
            ))?,
        };
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformation {
    action_list: Option<Vec<CybersourceActionsList>>,
    action_token_types: Option<Vec<CybersourceActionsTokenType>>,
    authorization_options: Option<CybersourceAuthorizationOptions>,
    commerce_indicator: CybersourceCommerceIndicator,
    capture: Option<bool>,
    capture_options: Option<CaptureOptions>,
    payment_solution: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceActionsList {
    TokenCreate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CybersourceActionsTokenType {
    InstrumentIdentifier,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthorizationOptions {
    initiator: CybersourcePaymentInitiator,
    merchant_intitiated_transaction: Option<MerchantInitiatedTransaction>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantInitiatedTransaction {
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentInitiator {
    #[serde(rename = "type")]
    initiator_type: Option<CybersourcePaymentInitiatorTypes>,
    credential_stored_on_file: Option<bool>,
    stored_credential_used: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CybersourcePaymentInitiatorTypes {
    Customer,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CybersourceCommerceIndicator {
    Internet,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOptions {
    capture_sequence_number: u32,
    total_capture_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentInformation {
    card: CardDetails,
    instrument_identifier: Option<CybersoucreInstrumentIdentifier>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCard {
    number: Secret<String>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    cryptogram: Secret<String>,
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayPaymentInformation {
    tokenized_card: TokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FluidData {
    value: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentInformation {
    fluid_data: FluidData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentInformation {
    Cards(CardPaymentInformation),
    GooglePay(GooglePayPaymentInformation),
    ApplePay(ApplePayPaymentInformation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CybersoucreInstrumentIdentifier {
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CardDetails {
    PaymentCard(Card),
    MandateCard(MandateCardDetails),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
    #[serde(rename = "type")]
    card_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandateCardDetails {
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: Option<BillTo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationIncrementalAuthorization {
    amount_details: AdditionalAmount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: String,
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalAmount {
    additional_amount: String,
    currency: String,
}

#[derive(Debug, Serialize)]
pub enum PaymentSolution {
    ApplePay,
    GooglePay,
}

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "1")]
    ApplePay,
}

impl From<PaymentSolution> for String {
    fn from(solution: PaymentSolution) -> Self {
        let payment_solution = match solution {
            PaymentSolution::ApplePay => "001",
            PaymentSolution::GooglePay => "012",
        };
        payment_solution.to_string()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Secret<String>,
    last_name: Secret<String>,
    address1: Secret<String>,
    locality: String,
    administrative_area: Secret<String>,
    postal_code: Secret<String>,
    country: api_enums::CountryAlpha2,
    email: pii::Email,
}

impl From<&CybersourceRouterData<&types::PaymentsAuthorizeRouterData>>
    for ClientReferenceInformation
{
    fn from(item: &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl
    From<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        Option<PaymentSolution>,
    )> for ProcessingInformation
{
    fn from(
        (item, solution): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            Option<PaymentSolution>,
        ),
    ) -> Self {
        let (action_list, action_token_types, authorization_options) =
            if item.router_data.request.setup_future_usage.is_some() {
                (
                    Some(vec![CybersourceActionsList::TokenCreate]),
                    Some(vec![CybersourceActionsTokenType::InstrumentIdentifier]),
                    Some(CybersourceAuthorizationOptions {
                        initiator: CybersourcePaymentInitiator {
                            initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                            credential_stored_on_file: Some(true),
                            stored_credential_used: None,
                        },
                        merchant_intitiated_transaction: None,
                    }),
                )
            } else {
                (None, None, None)
            };
        Self {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            payment_solution: solution.map(String::from),
            action_list,
            action_token_types,
            authorization_options,
            capture_options: None,
            commerce_indicator: CybersourceCommerceIndicator::Internet,
        }
    }
}

impl
    From<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        BillTo,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            BillTo,
        ),
    ) -> Self {
        Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency,
            },
            bill_to: Some(bill_to),
        }
    }
}

// for cybersource each item in Billing is mandatory
fn build_bill_to(
    address_details: &payments::Address,
    email: pii::Email,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let address = address_details
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;
    let mut state = address.to_state_code()?.peek().clone();
    state.truncate(20);
    Ok(BillTo {
        first_name: address.get_first_name()?.to_owned(),
        last_name: address.get_last_name()?.to_owned(),
        address1: address.get_line1()?.to_owned(),
        locality: address.get_city()?.to_owned(),
        administrative_area: Secret::from(state),
        postal_code: address.get_zip()?.to_owned(),
        country: address.get_country()?.to_owned(),
        email,
    })
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        payments::Card,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            payments::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));

        let card_issuer = ccard.get_card_issuer();
        let card_type = match card_issuer {
            Ok(issuer) => Some(String::from(issuer)),
            Err(_) => None,
        };

        let instrument_identifier =
            item.router_data
                .request
                .connector_mandate_id()
                .map(|mandate_token_id| CybersoucreInstrumentIdentifier {
                    id: mandate_token_id,
                });

        let card = if instrument_identifier.is_some() {
            CardDetails::MandateCard(MandateCardDetails {
                expiration_month: ccard.card_exp_month,
                expiration_year: ccard.card_exp_year,
            })
        } else {
            CardDetails::PaymentCard(Card {
                number: ccard.card_number,
                expiration_month: ccard.card_exp_month,
                expiration_year: ccard.card_exp_year,
                security_code: ccard.card_cvc,
                card_type,
            })
        };

        let payment_information = PaymentInformation::Cards(CardPaymentInformation {
            card,
            instrument_identifier,
        });

        let processing_information = ProcessingInformation::from((item, None));
        let client_reference_information = ClientReferenceInformation::from(item);

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
        })
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        Box<ApplePayPredecryptData>,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, apple_pay_data): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            Box<ApplePayPredecryptData>,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));
        let processing_information =
            ProcessingInformation::from((item, Some(PaymentSolution::ApplePay)));
        let client_reference_information = ClientReferenceInformation::from(item);
        let expiration_month = apple_pay_data.get_expiry_month()?;
        let expiration_year = apple_pay_data.get_four_digit_expiry_year()?;

        let payment_information = PaymentInformation::ApplePay(ApplePayPaymentInformation {
            tokenized_card: TokenizedCard {
                number: apple_pay_data.application_primary_account_number,
                cryptogram: apple_pay_data.payment_data.online_payment_cryptogram,
                transaction_type: TransactionType::ApplePay,
                expiration_year,
                expiration_month,
            },
        });

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
        })
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
        payments::GooglePayWalletData,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, google_pay_data): (
            &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
            payments::GooglePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));

        let payment_information = PaymentInformation::GooglePay(GooglePayPaymentInformation {
            fluid_data: FluidData {
                value: Secret::from(
                    consts::BASE64_ENGINE.encode(google_pay_data.tokenization_data.token),
                ),
            },
        });

        let processing_information =
            ProcessingInformation::from((item, Some(PaymentSolution::GooglePay)));
        let client_reference_information = ClientReferenceInformation::from(item);

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
        })
    }
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsAuthorizeRouterData>>
    for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            payments::PaymentMethodData::Card(ccard) => Self::try_from((item, ccard)),
            payments::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                payments::WalletData::ApplePay(_) => {
                    let payment_method_token = item.router_data.get_payment_method_token()?;
                    match payment_method_token {
                        types::PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                            Self::try_from((item, decrypt_data))
                        }
                        types::PaymentMethodToken::Token(_) => {
                            Err(errors::ConnectorError::InvalidWalletToken)?
                        }
                    }
                }
                payments::WalletData::GooglePay(google_pay_data) => {
                    Self::try_from((item, google_pay_data))
                }
                payments::WalletData::AliPayQr(_)
                | payments::WalletData::AliPayRedirect(_)
                | payments::WalletData::AliPayHkRedirect(_)
                | payments::WalletData::MomoRedirect(_)
                | payments::WalletData::KakaoPayRedirect(_)
                | payments::WalletData::GoPayRedirect(_)
                | payments::WalletData::GcashRedirect(_)
                | payments::WalletData::ApplePayRedirect(_)
                | payments::WalletData::ApplePayThirdPartySdk(_)
                | payments::WalletData::DanaRedirect {}
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
                | payments::WalletData::WeChatPayRedirect(_)
                | payments::WalletData::WeChatPayQr(_)
                | payments::WalletData::CashappQr(_)
                | payments::WalletData::SwishQr(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                )
                .into()),
            },
            payments::PaymentMethodData::CardRedirect(_)
            | payments::PaymentMethodData::PayLater(_)
            | payments::PaymentMethodData::BankRedirect(_)
            | payments::PaymentMethodData::BankDebit(_)
            | payments::PaymentMethodData::BankTransfer(_)
            | payments::PaymentMethodData::Crypto(_)
            | payments::PaymentMethodData::MandatePayment
            | payments::PaymentMethodData::Reward
            | payments::PaymentMethodData::Upi(_)
            | payments::PaymentMethodData::Voucher(_)
            | payments::PaymentMethodData::GiftCard(_)
            | payments::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsCaptureRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationWithBill,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsIncrementalAuthorizationRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationIncrementalAuthorization,
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsCaptureRouterData>>
    for CybersourcePaymentsCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            processing_information: ProcessingInformation {
                capture_options: Some(CaptureOptions {
                    capture_sequence_number: 1,
                    total_capture_count: 1,
                }),
                action_list: None,
                action_token_types: None,
                authorization_options: None,
                capture: None,
                commerce_indicator: CybersourceCommerceIndicator::Internet,
                payment_solution: None,
            },
            order_information: OrderInformationWithBill {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                },
                bill_to: None,
            },
        })
    }
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsIncrementalAuthorizationRouterData>>
    for CybersourcePaymentsIncrementalAuthorizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsIncrementalAuthorizationRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            processing_information: ProcessingInformation {
                action_list: None,
                action_token_types: None,
                authorization_options: Some(CybersourceAuthorizationOptions {
                    initiator: CybersourcePaymentInitiator {
                        initiator_type: None,
                        credential_stored_on_file: None,
                        stored_credential_used: Some(true),
                    },
                    merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                        reason: "5".to_owned(),
                    }),
                }),
                commerce_indicator: CybersourceCommerceIndicator::Internet,
                capture: None,
                capture_options: None,
                payment_solution: None,
            },
            order_information: OrderInformationIncrementalAuthorization {
                amount_details: AdditionalAmount {
                    additional_amount: item.amount.clone(),
                    currency: item.router_data.request.currency.to_string(),
                },
            },
        })
    }
}

pub struct CybersourceAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for CybersourceAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourcePaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    Voided,
    Reversed,
    Pending,
    Declined,
    Rejected,
    Challenge,
    AuthorizedPendingReview,
    AuthorizedRiskDeclined,
    Transmitted,
    InvalidRequest,
    ServerError,
    PendingAuthentication,
    PendingReview,
    //PartialAuthorized, not being consumed yet.
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceIncrementalAuthorizationStatus {
    Authorized,
    Declined,
    AuthorizedPendingReview,
}

impl ForeignFrom<(CybersourcePaymentStatus, bool)> for enums::AttemptStatus {
    fn foreign_from((status, capture): (CybersourcePaymentStatus, bool)) -> Self {
        match status {
            CybersourcePaymentStatus::Authorized
            | CybersourcePaymentStatus::AuthorizedPendingReview => {
                if capture {
                    // Because Cybersource will return Payment Status as Authorized even in AutoCapture Payment
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            CybersourcePaymentStatus::Pending => {
                if capture {
                    Self::Charged
                } else {
                    Self::Pending
                }
            }
            CybersourcePaymentStatus::Succeeded | CybersourcePaymentStatus::Transmitted => {
                Self::Charged
            }
            CybersourcePaymentStatus::Voided | CybersourcePaymentStatus::Reversed => Self::Voided,
            CybersourcePaymentStatus::Failed
            | CybersourcePaymentStatus::Declined
            | CybersourcePaymentStatus::AuthorizedRiskDeclined
            | CybersourcePaymentStatus::Rejected
            | CybersourcePaymentStatus::InvalidRequest
            | CybersourcePaymentStatus::ServerError => Self::Failure,
            CybersourcePaymentStatus::PendingAuthentication => Self::AuthenticationPending,
            CybersourcePaymentStatus::PendingReview | CybersourcePaymentStatus::Challenge => {
                Self::Pending
            }
        }
    }
}

impl From<CybersourceIncrementalAuthorizationStatus> for common_enums::AuthorizationStatus {
    fn from(item: CybersourceIncrementalAuthorizationStatus) -> Self {
        match item {
            CybersourceIncrementalAuthorizationStatus::Authorized
            | CybersourceIncrementalAuthorizationStatus::AuthorizedPendingReview => Self::Success,
            CybersourceIncrementalAuthorizationStatus::Declined => Self::Failure,
        }
    }
}

impl From<CybersourcePaymentStatus> for enums::RefundStatus {
    fn from(item: CybersourcePaymentStatus) -> Self {
        match item {
            CybersourcePaymentStatus::Succeeded | CybersourcePaymentStatus::Transmitted => {
                Self::Success
            }
            CybersourcePaymentStatus::Failed => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CybersourcePaymentsResponse {
    ClientReferenceInformation(CybersourceClientReferenceResponse),
    ErrorInformation(CybersourceErrorInformationResponse),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceClientReferenceResponse {
    id: String,
    status: CybersourcePaymentStatus,
    client_reference_information: ClientReferenceInformation,
    token_information: Option<CybersourceTokenInformation>,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceErrorInformationResponse {
    id: String,
    error_information: CybersourceErrorInformation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsIncrementalAuthorizationResponse {
    status: CybersourceIncrementalAuthorizationStatus,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceSetupMandatesResponse {
    id: String,
    status: CybersourcePaymentStatus,
    error_information: Option<CybersourceErrorInformation>,
    client_reference_information: Option<ClientReferenceInformation>,
    token_information: Option<CybersourceTokenInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReferenceInformation {
    code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceTokenInformation {
    instrument_identifier: CybersoucreInstrumentIdentifier,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CybersourceErrorInformation {
    reason: Option<String>,
    message: Option<String>,
}

impl<F, T>
    From<(
        &CybersourceErrorInformationResponse,
        types::ResponseRouterData<F, CybersourcePaymentsResponse, T, types::PaymentsResponseData>,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    fn from(
        (error_response, item): (
            &CybersourceErrorInformationResponse,
            types::ResponseRouterData<
                F,
                CybersourcePaymentsResponse,
                T,
                types::PaymentsResponseData,
            >,
        ),
    ) -> Self {
        Self {
            response: Err(types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: error_response
                    .error_information
                    .message
                    .clone()
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                reason: error_response.error_information.reason.clone(),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(error_response.id.clone()),
            }),
            ..item.data
        }
    }
}

fn get_error_response_if_failure(
    (info_response, status, http_code): (
        &CybersourceClientReferenceResponse,
        enums::AttemptStatus,
        u16,
    ),
) -> Option<types::ErrorResponse> {
    if is_payment_failure(status) {
        let (message, reason) = match info_response.error_information.as_ref() {
            Some(error_info) => (
                error_info
                    .message
                    .clone()
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                error_info.reason.clone(),
            ),
            None => (consts::NO_ERROR_MESSAGE.to_string(), None),
        };

        Some(types::ErrorResponse {
            code: consts::NO_ERROR_CODE.to_string(),
            message,
            reason,
            status_code: http_code,
            attempt_status: Some(enums::AttemptStatus::Failure),
            connector_transaction_id: Some(info_response.id.clone()),
        })
    } else {
        None
    }
}

fn is_payment_failure(status: enums::AttemptStatus) -> bool {
    match status {
        common_enums::AttemptStatus::AuthenticationFailed
        | common_enums::AttemptStatus::AuthorizationFailed
        | common_enums::AttemptStatus::CaptureFailed
        | common_enums::AttemptStatus::VoidFailed
        | common_enums::AttemptStatus::Failure => true,
        common_enums::AttemptStatus::Started
        | common_enums::AttemptStatus::RouterDeclined
        | common_enums::AttemptStatus::AuthenticationPending
        | common_enums::AttemptStatus::AuthenticationSuccessful
        | common_enums::AttemptStatus::Authorized
        | common_enums::AttemptStatus::Charged
        | common_enums::AttemptStatus::Authorizing
        | common_enums::AttemptStatus::CodInitiated
        | common_enums::AttemptStatus::Voided
        | common_enums::AttemptStatus::VoidInitiated
        | common_enums::AttemptStatus::CaptureInitiated
        | common_enums::AttemptStatus::AutoRefunded
        | common_enums::AttemptStatus::PartialCharged
        | common_enums::AttemptStatus::PartialChargedAndChargeable
        | common_enums::AttemptStatus::Unresolved
        | common_enums::AttemptStatus::Pending
        | common_enums::AttemptStatus::PaymentMethodAwaited
        | common_enums::AttemptStatus::ConfirmationAwaited
        | common_enums::AttemptStatus::DeviceDataCollectionPending => false,
    }
}

fn get_payment_response(
    (info_response, status, http_code): (
        &CybersourceClientReferenceResponse,
        enums::AttemptStatus,
        u16,
    ),
) -> Result<types::PaymentsResponseData, types::ErrorResponse> {
    let error_response = get_error_response_if_failure((info_response, status, http_code));
    match error_response {
        Some(error) => Err(error),
        None => {
            let incremental_authorization_allowed =
                Some(status == enums::AttemptStatus::Authorized);
            let mandate_reference =
                info_response
                    .token_information
                    .clone()
                    .map(|token_info| types::MandateReference {
                        connector_mandate_id: Some(token_info.instrument_identifier.id),
                        payment_method_id: None,
                    });
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(info_response.id.clone()),
                redirection_data: None,
                mandate_reference,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    info_response
                        .client_reference_information
                        .code
                        .clone()
                        .unwrap_or(info_response.id.clone()),
                ),
                incremental_authorization_allowed,
            })
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status = enums::AttemptStatus::foreign_from((
                    info_response.status.clone(),
                    item.data.request.is_auto_capture()?,
                ));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            CybersourcePaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(Self::from((&error_response.clone(), item)))
            }
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status =
                    enums::AttemptStatus::foreign_from((info_response.status.clone(), true));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            CybersourcePaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(Self::from((&error_response.clone(), item)))
            }
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCancelData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsCancelData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            types::PaymentsCancelData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePaymentsResponse::ClientReferenceInformation(info_response) => {
                let status =
                    enums::AttemptStatus::foreign_from((info_response.status.clone(), false));
                let response = get_payment_response((&info_response, status, item.http_code));
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            }
            CybersourcePaymentsResponse::ErrorInformation(ref error_response) => {
                Ok(Self::from((&error_response.clone(), item)))
            }
        }
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourceSetupMandatesResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourceSetupMandatesResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let mandate_reference =
            item.response
                .token_information
                .map(|token_info| types::MandateReference {
                    connector_mandate_id: Some(token_info.instrument_identifier.id),
                    payment_method_id: None,
                });
        let mut mandate_status = enums::AttemptStatus::foreign_from((item.response.status, false));
        if matches!(mandate_status, enums::AttemptStatus::Authorized) {
            //In case of zero auth mandates we want to make the payment reach the terminal status so we are converting the authorized status to charged as well.
            mandate_status = enums::AttemptStatus::Charged
        }
        Ok(Self {
            status: mandate_status,
            response: match item.response.error_information {
                Some(error) => Err(types::ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: error
                        .message
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: error.reason,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.id),
                }),
                _ => Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.id.clone(),
                    ),
                    redirection_data: None,
                    mandate_reference,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: item
                        .response
                        .client_reference_information
                        .map(|cref| cref.code)
                        .unwrap_or(Some(item.response.id)),
                    incremental_authorization_allowed: Some(
                        mandate_status == enums::AttemptStatus::Authorized,
                    ),
                }),
            },
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<
            F,
            CybersourcePaymentsIncrementalAuthorizationResponse,
            T,
            types::PaymentsResponseData,
        >,
        bool,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            types::ResponseRouterData<
                F,
                CybersourcePaymentsIncrementalAuthorizationResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        Ok(Self {
            response: match item.response.error_information {
                Some(error) => Ok(
                    types::PaymentsResponseData::IncrementalAuthorizationResponse {
                        status: common_enums::AuthorizationStatus::Failure,
                        error_code: error.reason,
                        error_message: error.message,
                        connector_authorization_id: None,
                    },
                ),
                _ => Ok(
                    types::PaymentsResponseData::IncrementalAuthorizationResponse {
                        status: item.response.status.into(),
                        error_code: None,
                        error_message: None,
                        connector_authorization_id: None,
                    },
                ),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CybersourceTransactionResponse {
    ApplicationInformation(CybersourceApplicationInfoResponse),
    ErrorInformation(CybersourceErrorInformationResponse),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceApplicationInfoResponse {
    id: String,
    application_information: ApplicationInformation,
    client_reference_information: Option<ClientReferenceInformation>,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: CybersourcePaymentStatus,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CybersourceTransactionResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsSyncData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CybersourceTransactionResponse,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourceTransactionResponse::ApplicationInformation(app_response) => {
                let status = enums::AttemptStatus::foreign_from((
                    app_response.application_information.status,
                    item.data.request.is_auto_capture()?,
                ));
                let incremental_authorization_allowed =
                    Some(status == enums::AttemptStatus::Authorized);
                if is_payment_failure(status) {
                    let (message, reason) = match app_response.error_information {
                        Some(error_info) => (
                            error_info
                                .message
                                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                            error_info.reason,
                        ),
                        None => (consts::NO_ERROR_MESSAGE.to_string(), None),
                    };
                    Ok(Self {
                        response: Err(types::ErrorResponse {
                            code: consts::NO_ERROR_CODE.to_string(),
                            message,
                            reason,
                            status_code: item.http_code,
                            attempt_status: Some(enums::AttemptStatus::Failure),
                            connector_transaction_id: Some(app_response.id),
                        }),
                        status: enums::AttemptStatus::Failure,
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                app_response.id.clone(),
                            ),
                            redirection_data: None,
                            mandate_reference: None,
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: app_response
                                .client_reference_information
                                .map(|cref| cref.code)
                                .unwrap_or(Some(app_response.id)),
                            incremental_authorization_allowed,
                        }),
                        ..item.data
                    })
                }
            }
            CybersourceTransactionResponse::ErrorInformation(error_response) => Ok(Self {
                status: item.data.status,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        error_response.id.clone(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(error_response.id),
                    incremental_authorization_allowed: None,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRefundRequest {
    order_information: OrderInformation,
}

impl<F> TryFrom<&CybersourceRouterData<&types::RefundsRouterData<F>>> for CybersourceRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                },
            },
        })
    }
}

impl From<CybersourceRefundStatus> for enums::RefundStatus {
    fn from(item: CybersourceRefundStatus) -> Self {
        match item {
            CybersourceRefundStatus::Succeeded | CybersourceRefundStatus::Transmitted => {
                Self::Success
            }
            CybersourceRefundStatus::Failed | CybersourceRefundStatus::Voided => Self::Failure,
            CybersourceRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceRefundStatus {
    Succeeded,
    Transmitted,
    Failed,
    Pending,
    Voided,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRefundResponse {
    id: String,
    status: CybersourceRefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, CybersourceRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, CybersourceRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsyncApplicationInformation {
    status: CybersourceRefundStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRsyncResponse {
    id: String,
    application_information: RsyncApplicationInformation,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CybersourceRsyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, CybersourceRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(
                    item.response.application_information.status,
                ),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceStandardErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize)]
pub struct CybersourceAuthenticationErrorResponse {
    pub response: AuthenticationErrorInformation,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CybersourceErrorResponse {
    StandardError(CybersourceStandardErrorResponse),
    AuthenticationError(CybersourceAuthenticationErrorResponse),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct AuthenticationErrorInformation {
    pub rmsg: String,
}
