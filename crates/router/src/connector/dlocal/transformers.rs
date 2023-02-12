use crate::{
    core::errors,
    types::{self, api, storage::enums},
};
use common_utils::pii;
use serde::{Deserialize, Serialize};
use masking::PeekInterface;
use crate::pii::Secret;

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DlocalPaymentsRequest {
    amount: i64,
    currency: String,
    country: String,
    payment_method_flow: String,
    payment_method_id: String,
    payer: PayerType,
    card: CardData,
    order_id: String,
    notification_url: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Amount {
    amount: i64,
    currency: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CardData {
    holder_name: Secret<String>,
    number: Secret<String, pii::CardNumber>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    cvv: Secret<String>,
    capture: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AddressType {
    country: String,
    state: String,
    city: String,
    zip_code: String,
    street: String,
    number: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayerType {
    name: String,
    email: String,
    document: String,
    phone: String,
    address: AddressType,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for DlocalPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let dcapture = match item.request.capture_method {
            Some(a) => match a {
                storage_models::enums::CaptureMethod::Automatic => true,
                storage_models::enums::CaptureMethod::Manual => false,
                storage_models::enums::CaptureMethod::ManualMultiple => false,
                storage_models::enums::CaptureMethod::Scheduled => false,
            },
            None => false,
        };
        let card_data = match item.request.payment_method_data {
            api_models::payments::PaymentMethod::Card(ref ccard) => CardData {
                holder_name: ccard.card_holder_name.to_owned(),
                number: ccard.card_number.to_owned(),
                cvv: ccard.card_cvc.to_owned(),
                expiration_month: ccard.card_exp_month.to_owned(),
                expiration_year: ccard.card_exp_year.to_owned(),
                capture: dcapture.to_string(),
            },
            _ => CardData {
                holder_name: "dummy".to_string().into(),
                number: "kdajf".to_owned().into(),
                cvv: "232".to_owned().into(),
                expiration_month: "11".to_owned().into(),
                expiration_year: "2030".to_owned().into(),
                capture: dcapture.to_string(),
            },
        };
        Ok(Self {
            amount: item.request.amount,
            currency: "BRL".to_owned(),
            country: "BR".to_owned(),
            payment_method_flow: "DIRECT".to_owned(),
            payment_method_id: "CARD".to_owned(),

            payer: PayerType {
                name: "dhanuh".to_string(),
                email: match &item.request.email {
                    Some(a) => a.peek().clone().to_string(),
                    None => "dlocal@gmail.com".to_string().into(),
                },
                document: "48230764433".to_owned(),
                phone: "4832695312".to_owned(),
                address: AddressType {
                    country: "BR".to_owned(),
                    state: "Santa Catarina".to_owned(),
                    city: "Florianopolis".to_owned(),
                    zip_code: "88058".to_owned(),
                    street: "Rodovia Armando Calil Bulos".to_owned(),
                    number: "5940".to_owned(),
                },
            },
            card: card_data,

            order_id: item.payment_id.to_owned(),
            notification_url: "https://postman-echo.com/post".to_owned(),
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct DlocalAuthType {
    pub(super) api_key: String,    //transkey
    pub(super) login_key: String,  //login-key
    pub(super) api_secret: String, // secret-key
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
                api_key: api_key.to_string(),
                login_key: key1.to_string(),
                api_secret: api_secret.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DlocalPaymentStatus {
    Authorized,
    Paid,
    Verified,
    Cancelled,
    #[default]
    Pending,
}

impl From<DlocalPaymentStatus> for enums::AttemptStatus {
    fn from(item: DlocalPaymentStatus) -> Self {
        match item {
            DlocalPaymentStatus::Authorized => Self::Authorized,
            DlocalPaymentStatus::Paid => Self::Charged,
            DlocalPaymentStatus::Verified => Self::Authorized,
            DlocalPaymentStatus::Cancelled => Self::Voided,
            DlocalPaymentStatus::Pending => Self::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsResponse {
    status: DlocalPaymentStatus,
    id: String,
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
