use base64::Engine;
use serde::{Deserialize, Serialize};
use masking::Secret;
use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    types::{self,api, storage::enums}
};


#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FortePaymentsRequest {
    #[serde(flatten)]
    card: ForteCard,
    authorization_amount: i64,
    subtotal_amount: i64,
    billing_address: Address,
}

#[derive(Debug, Serialize)]
pub struct Address {
    first_name : String,
    second_name : String,
    physical_address : Option<PhysicalAddress>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicalAddress{
    street_line1: String,
    street_line2: String,
    locality: String,
    region: String,
    country: String,
    postal_code: String,
  }


#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ForteCard {
    card_type: String,
    name_on_card: Secret<String>,
    account_number: Secret<String, pii::CardNumber>,
    expire_month: Secret<String>,
    expire_year: Secret<String>,
    card_verification_value: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let auth_mode = match item.request.capture_method {
            Some(enums::CaptureMethod::Manual) => ForteTxnType::Manual,
            _ => ForteTxnType::Automatic,
        };

        let payment_method = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => Ok(ForteCard {
                card_type: ccard.card_brand.clone(),
                name_on_card: ccard.name_on_card.clone(),
                account_number: ccard.card_number,
                expire_month: ccard.card_exp_month.clone(),
                expire_year: ccard.card_exp_year.clone(),
                card_verification_value: ccard.card_cvc,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "payment method".to_string(),
            )),        
    }?;

    //Must change
    let mut name = item.request.payment_method_data.clone().name_on_card.clone().split(' ');
    let first_name = name.next().unwrap().to_string();
    let second_name = name.next().unwrap().to_string();

    ok(Self {
        payment_method,
        authorization_amount: item.request.amount,
        billing_address: Address{
            first_name,
            second_name,
        }
    })
    }
}


pub struct ForteAuthType {
    pub(super) api_key : String,
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey { api_key, key1, api_secret } = auth_type {
            let auth_key = format!("{api_key}:{api_secret}");
            let auth_header = format!("Base {}", consts::BASE64_ENGINE.encode(auth_key));
            Ok(Self {
                api_key: auth_header,
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FortePaymentStatus {
    Authorized,
    #[default]
    Ready,
    #[serde(rename = "Card Verified")]
    Complete,
    Voided,
    Declined,
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Authorized => enums::AttemptStatus::Charged,
            FortePaymentStatus::Declined => enums::AttemptStatus::Failure,
            FortePaymentStatus::Failed => enums::AttemptStatus::Failure,
            FortePaymentStatus::Ready => enums::AttemptStatus::Authorizing,
            FortePaymentStatus::Complete => enums::AttemptStatus::Pending,
            FortePaymentStatus::Voided => enums::AttemptStatus::Voided,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentsResponse {
        #[serde(deserialize_with = "str_or_i32")]
        transaction_id: String,
        location_id: String,
        action: ForteAction,
        status: Option<FortePaymentStatus>,
        received_date: Option<String>,
        masked_account_number: Option<String>,
        organization_id: Option<String>,
        authorization_amount: f64,
        authorization_code : String,
        entered_by: Option<String>,
        billing_address: Optional<Address>,
        #[serde(flatten)]
        card: CardData,
        #[serde(flatten)]
        response: FortePaymentResponseData,
        #[serde(flatten)]
        links: Links,
}

//Card Implementation repeated: Problem => expire_month data type vary. 
//Solution yet to be sorted
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardData {
    name_on_card: String,
    last_four: String,
    expire_month: i32,
    expire_year: i32,
    card_type: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentResponseData {
    environment : String,
    response_type: String,
    response_code: ForteTransactionResponseCodes,
    response_desc: String,
    authorization_code: String,
    avs_result: String,
    cvv_result: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Links {
    disputes: String,
    settlements: String,
    #[serde(rename = "self")]
    self_url: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        if item.response.status != None {
            let status = enums::Status::from(item.response.response.status);
        } else {
            let status = match item.response.response.response_code {
                Some("A01") => enums::AttemptStatus::Charged,
                Some("A03") => enums::AttemptStatus::Authorized,
                Some("X02") => enums::AttemptStatus::Voided,
                Some(other) => match other.chars().next() {
                    Some("U") => enums::AttemptStatus::Failure,
                    Some("F") => enums::AttemptStatus::Pending,
                },
                _ => enums::AttemptStatus::Pending,
            };
        }
        
        ok( Self{
            id: Some(item.response.transaction_id),
            amount_received: Some(item.response.response.authorization_amount),
            status,
            response: Ok(types::PaymentsResponseData::SessionTokenResponse {
                session_token: item.response.response.authorization_code,
            }),
            ..item.data
        })
    }
}


//Void
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForteMeta {
    session_token: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ForteVoidRequest {
    #[serde(flatten)]
    action: ForteAction,
    authorization_code: String,
}



impl TryFrom<&types::PaymentsCancelRouterData> for ForteVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let action = ForteAction::Capture;
        let meta: ForteMeta = utils::to_connector_meta(value.request.connector_meta.clone())?;
        Ok(Self {
            action,
            authorization_code: meta.session_token,
        })
    }
}

//Capture

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForteCaptureRequest {
    #[serde(flatten)]
    action: ForteAction,
    transaction_id: String,
    authorization_amount: i64,
    authorization_code: String,//can make it secrte
}

impl TryFrom<&types::PaymentsCaptureRouterData> for ForteCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let action = ForteAction::Capture;
        let transaction_id = item.request.connector_transaction_id.to_string();
        let authorization_amount = (item.request.amount_to_capture)?;
    
        let meta: ForteMeta = utils::to_connector_meta(value.request.connector_meta.clone())?;
        
        Ok(Self {
            action,
            transaction_id,
            authorization_amount,
            authorization_code: meta.session_token,
        })
    }
}


//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {
    pub amount: i64
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.request.amount,
        })
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
pub struct RefundResponse {
    id: String,
    status: RefundStatus
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ForteErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowerCase")]
pub enum ForteAction {
    Capture,
    Authorize,
    Force,
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ForteTxnType {
    Manual,
    Automatic,
}