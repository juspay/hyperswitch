use api_models::payments;
use common_utils::pii;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, PaymentsAuthorizeRequestData, PaymentsSetupMandateRequestData,
        PhoneDetailsData, RouterData,
    },
    consts,
    core::errors,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums,
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
        let phone = item.get_billing_phone()?;
        let number_with_code = phone.get_number_with_country_code()?;
        let email = item.request.get_email()?;
        let bill_to = build_bill_to(item.get_billing()?, email, number_with_code)?;

        let order_information = OrderInformationWithBill {
            amount_details: Amount {
                total_amount: "0".to_string(),
                currency: item.request.currency.to_string(),
            },
            bill_to: Some(bill_to),
        };
        let (action_list, action_token_types, authorization_options) = (
            Some(vec![CybersourceActionsList::TokenCreate]),
            Some(vec![CybersourceActionsTokenType::InstrumentIdentifier]),
            Some(CybersourceAuthorizationOptions {
                initiator: CybersourcePaymentInitiator {
                    initiator_type: CybersourcePaymentInitiatorTypes::Customer,
                    credential_stored_on_file: true,
                },
            }),
        );

        let processing_information = ProcessingInformation {
            capture: Some(false),
            capture_options: None,
            action_list,
            action_token_types,
            authorization_options,
            commerce_indicator: CybersourceCommerceIndicator::Internet,
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
                });
                PaymentInformation {
                    card,
                    instrument_identifier: None,
                }
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentInitiator {
    #[serde(rename = "type")]
    initiator_type: CybersourcePaymentInitiatorTypes,
    credential_stored_on_file: bool,
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
pub struct PaymentInformation {
    card: CardDetails,
    instrument_identifier: Option<CybersoucreInstrumentIdentifier>,
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
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: String,
    currency: String,
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
    phone_number: Secret<String>,
}

// for cybersource each item in Billing is mandatory
fn build_bill_to(
    address_details: &payments::Address,
    email: pii::Email,
    phone_number: Secret<String>,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let address = address_details
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;
    Ok(BillTo {
        first_name: address.get_first_name()?.to_owned(),
        last_name: address.get_last_name()?.to_owned(),
        address1: address.get_line1()?.to_owned(),
        locality: address.get_city()?.to_owned(),
        administrative_area: address.to_state_code()?,
        postal_code: address.get_zip()?.to_owned(),
        country: address.get_country()?.to_owned(),
        email,
        phone_number,
    })
}

impl TryFrom<&CybersourceRouterData<&types::PaymentsAuthorizeRouterData>>
    for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let phone = item.router_data.get_billing_phone()?;
        let number_with_code = phone.get_number_with_country_code()?;
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_billing()?, email, number_with_code)?;

        let order_information = OrderInformationWithBill {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
            },
            bill_to: Some(bill_to),
        };
        let (action_list, action_token_types, authorization_options) =
            if item.router_data.request.setup_future_usage.is_some() {
                (
                    Some(vec![CybersourceActionsList::TokenCreate]),
                    Some(vec![CybersourceActionsTokenType::InstrumentIdentifier]),
                    Some(CybersourceAuthorizationOptions {
                        initiator: CybersourcePaymentInitiator {
                            initiator_type: CybersourcePaymentInitiatorTypes::Customer,
                            credential_stored_on_file: true,
                        },
                    }),
                )
            } else {
                (None, None, None)
            };

        let processing_information = ProcessingInformation {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            capture_options: None,
            action_list,
            action_token_types,
            authorization_options,
            commerce_indicator: CybersourceCommerceIndicator::Internet,
        };

        let client_reference_information = ClientReferenceInformation {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        };
        let payment_information = match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
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
                    })
                };
                PaymentInformation {
                    card,
                    instrument_identifier,
                }
            }
            payments::PaymentMethodData::CardRedirect(_)
            | payments::PaymentMethodData::Wallet(_)
            | payments::PaymentMethodData::PayLater(_)
            | payments::PaymentMethodData::BankRedirect(_)
            | payments::PaymentMethodData::BankDebit(_)
            | payments::PaymentMethodData::BankTransfer(_)
            | payments::PaymentMethodData::Crypto(_)
            | payments::PaymentMethodData::MandatePayment
            | payments::PaymentMethodData::Reward
            | payments::PaymentMethodData::Upi(_)
            | payments::PaymentMethodData::Voucher(_)
            | payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                ))?
            }
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
pub struct CybersourcePaymentsCaptureRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationWithBill,
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
            },
            order_information: OrderInformationWithBill {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency.to_string(),
                },
                bill_to: None,
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
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourcePaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    Voided,
    Reversed,
    Pending,
    Declined,
    AuthorizedPendingReview,
    Transmitted,
    #[default]
    Processing,
}

impl From<CybersourcePaymentStatus> for enums::AttemptStatus {
    fn from(item: CybersourcePaymentStatus) -> Self {
        match item {
            CybersourcePaymentStatus::Authorized
            | CybersourcePaymentStatus::AuthorizedPendingReview => Self::Authorized,
            CybersourcePaymentStatus::Succeeded | CybersourcePaymentStatus::Transmitted => {
                Self::Charged
            }
            CybersourcePaymentStatus::Voided | CybersourcePaymentStatus::Reversed => Self::Voided,
            CybersourcePaymentStatus::Failed | CybersourcePaymentStatus::Declined => Self::Failure,
            CybersourcePaymentStatus::Processing => Self::Authorizing,
            CybersourcePaymentStatus::Pending => Self::Pending,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsResponse {
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
    reason: String,
    message: String,
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<F, CybersourcePaymentsResponse, T, types::PaymentsResponseData>,
        bool,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            types::ResponseRouterData<
                F,
                CybersourcePaymentsResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let is_capture = data.1;
        let mandate_reference =
            item.response
                .token_information
                .map(|token_info| types::MandateReference {
                    connector_mandate_id: Some(token_info.instrument_identifier.id),
                    payment_method_id: None,
                });
        Ok(Self {
            status: get_payment_status(is_capture, item.response.status.into()),
            response: match item.response.error_information {
                Some(error) => Err(types::ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: error.message,
                    reason: Some(error.reason),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
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
                }),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceTransactionResponse {
    id: String,
    application_information: ApplicationInformation,
    client_reference_information: Option<ClientReferenceInformation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: CybersourcePaymentStatus,
}

fn get_payment_status(is_capture: bool, status: enums::AttemptStatus) -> enums::AttemptStatus {
    let is_authorized = matches!(status, enums::AttemptStatus::Authorized);
    if is_capture && is_authorized {
        return enums::AttemptStatus::Pending;
    }
    status
}

impl<F, T>
    TryFrom<(
        types::ResponseRouterData<
            F,
            CybersourceTransactionResponse,
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
                CybersourceTransactionResponse,
                T,
                types::PaymentsResponseData,
            >,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let is_capture = data.1;
        Ok(Self {
            status: get_payment_status(
                is_capture,
                item.response.application_information.status.into(),
            ),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .client_reference_information
                    .map(|cref| cref.code)
                    .unwrap_or(Some(item.response.id)),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<Reason>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize, strum::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Reason {
    MissingField,
    InvalidData,
    DuplicateRequest,
    InvalidCard,
    AuthAlreadyReversed,
    CardTypeNotAccepted,
    InvalidMerchantConfiguration,
    ProcessorUnavailable,
    InvalidAmount,
    InvalidCardType,
    InvalidPaymentId,
    NotSupported,
    SystemError,
    ServerTimeout,
    ServiceTimeout,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
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
                    currency: item.router_data.request.currency.to_string(),
                },
            },
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, CybersourcePaymentsResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, CybersourcePaymentsResponse>,
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CybersourceTransactionResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, CybersourceTransactionResponse>,
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
