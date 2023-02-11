use crate::{
    connector::utils::{self, AddressDetailsData, PaymentsRequestData},
    core::errors,
    types::{self, api, storage::enums},
};
use api_models::payments;
use common_utils::pii;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethodId {
    #[default]
    Card,
    // BankTransfer,
    // Wallet
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethodFlow {
    #[default]
    Direct,
    // Redirect
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct Payer {
    name: Secret<String>,
    email: Secret<String, pii::Email>,
    // birth_date: Secret<String>,
    // phone: Secret<String>,
    document: Secret<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct Card {
    holder_name: String,
    expiration_month: i32,
    expiration_year: i32,
    number: String,
    cvv: String,
    capture: bool,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct DlocalPaymentsRequest {
    amount: i64,
    currency: String,
    payment_method_id: PaymentMethodId,
    payment_method_flow: PaymentMethodFlow,
    country: String,
    payer: Payer,
    card: Card,
    order_id: String,
}

fn build_payer(
    address_details: &payments::Address,
    email: Secret<String, pii::Email>,
) -> Result<Payer, error_stack::Report<errors::ConnectorError>> {
    let address = address_details
        .address
        .as_ref()
        .ok_or_else(utils::missing_field_err("billing.address"))?;

    let payer_name = format!(
        "{} {}",
        address.get_first_name()?.peek(),
        address.get_last_name()?.peek()
    )
    .into();

    let payer_document = "23199364160".to_string().into(); // Hard-coding user document

    Ok(Payer {
        name: payer_name,
        email,
        document: payer_document,
    })
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for DlocalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                // let payment_method_id = match item.request.payment_method_data {
                //     Card => PaymentMethodId::Card,
                //     BankTransfer => PaymentMethodId::BankTransfer,
                //     Wallet => PaymentMethodId::Wallet
                // };

                // let payment_method_flow = match item.request.payment_method_data {
                //     Card => PaymentMethodFlow::Direct,
                //     BankTransfer => PaymentMethodFlow::Redirect,
                //     Wallet => PaymentMethodFlow::Redirect
                // };

                let payment_method_id = PaymentMethodId::Card;
                let payment_method_flow = PaymentMethodFlow::Direct;

                let address_details = item.get_billing()?;
                let payer_email = item
                    .request
                    .email
                    .clone()
                    .ok_or_else(utils::missing_field_err("email"))?;

                let is_auto_capture = item.request.capture_method
                    == Some(storage_models::enums::CaptureMethod::Automatic);

                Ok(Self {
                    amount: item.request.amount,
                    currency: item.request.currency.to_string(),
                    payment_method_id,
                    payment_method_flow,
                    country: "BR".to_string(), // Hard-coding Brazil for country
                    payer: build_payer(address_details, payer_email)?,
                    card: Card {
                        number: ccard.card_number.peek().clone(),
                        holder_name: ccard.card_holder_name.peek().clone(),
                        expiration_month: ccard
                            .card_exp_month
                            .peek()
                            .clone()
                            .parse::<i32>()
                            .unwrap(),
                        expiration_year: ccard.card_exp_year.peek().clone().parse::<i32>().unwrap(),
                        cvv: ccard.card_cvc.peek().clone(),
                        capture: is_auto_capture, // True for authorize
                    },
                    order_id: item
                        .attempt_id
                        .clone()
                        .ok_or_else(utils::missing_field_err("attempt_id"))?,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct DlocalAuthType {
    pub(super) x_login: String,
    pub(super) x_trans_key: String,
    pub(super) secret_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for DlocalAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                x_login: api_key.to_string(),
                x_trans_key: key1.to_string(),
                secret_key: api_secret.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DlocalPaymentStatus {
    Paid,
    #[default]
    Pending,
    Rejected,
    Cancelled,
    Authorized,
    Verified,
}

impl From<DlocalPaymentStatus> for enums::AttemptStatus {
    fn from(item: DlocalPaymentStatus) -> Self {
        match item {
            DlocalPaymentStatus::Paid => Self::Charged,
            DlocalPaymentStatus::Rejected => Self::Failure,
            DlocalPaymentStatus::Cancelled => Self::RouterDeclined,
            DlocalPaymentStatus::Authorized => Self::Authorized,
            DlocalPaymentStatus::Verified => Self::Charged,
            DlocalPaymentStatus::Pending => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsResponse {
    status: DlocalPaymentStatus,
    id: String,
    amount: i32,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct DlocalRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for DlocalRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        todo!()
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DlocalErrorResponse {}
