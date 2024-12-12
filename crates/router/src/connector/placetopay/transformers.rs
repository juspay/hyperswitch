use common_utils::{date_time, request::Method, types::MinorUnit};
use diesel_models::enums;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use ring::digest;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, BrowserInformationData, CardData, PaymentsAuthorizeRequestData,
        PaymentsSyncRequestData, RouterData,
    },
    consts,
    core::errors,
    services,
    types::{self, api, domain, storage::enums as storage_enums},
};

pub struct PlacetopayRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PlacetopayRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayPaymentsRequest {
    auth: PlacetopayAuth,
    payment: PlacetopayPayment,
    instrument: PlacetopayInstrument,
    ip_address: Secret<String, common_utils::pii::IpAddress>,
    user_agent: String,
    return_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacetopayAuthorizeAction {
    Checkin,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayAuthType {
    login: Secret<String>,
    tran_key: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayAuth {
    login: Secret<String>,
    tran_key: Secret<String>,
    nonce: Secret<String>,
    seed: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayPayment {
    reference: String,
    description: String,
    amount: PlacetopayAmount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayAmount {
    currency: storage_enums::Currency,
    total: MinorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayInstrument {
    card: PlacetopayCard,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayCard {
    number: cards::CardNumber,
    expiration: Secret<String>,
    cvv: Secret<String>,
}

impl TryFrom<&PlacetopayRouterData<&types::PaymentsAuthorizeRouterData>>
    for PlacetopayPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PlacetopayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let browser_info = item.router_data.request.get_browser_info()?;
        let ip_address = browser_info.get_ip_address()?;
        let user_agent = browser_info.get_user_agent()?;
        let auth = PlacetopayAuth::try_from(&item.router_data.connector_auth_type)?;
        let payment = PlacetopayPayment {
            reference: item.router_data.connector_request_reference_id.clone(),
            description: item.router_data.get_description()?,
            amount: PlacetopayAmount {
                currency: item.router_data.request.currency,
                total: item.amount,
            },
        };
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(req_card) => {
                let card = PlacetopayCard {
                    number: req_card.card_number.clone(),
                    expiration: req_card
                        .clone()
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned())?,
                    cvv: req_card.card_cvc.clone(),
                };
                let return_url = item.router_data.request.complete_authorize_url.clone();
                Ok(Self {
                    ip_address,
                    user_agent,
                    auth,
                    payment,
                    instrument: PlacetopayInstrument {
                        card: card.to_owned(),
                    },
                    return_url,
                })
            }
            domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::RealTimePayment(_)
            | domain::PaymentMethodData::MobilePayment(_)
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::OpenBanking(_)
            | domain::PaymentMethodData::CardToken(_)
            | domain::PaymentMethodData::NetworkToken(_)
            | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Placetopay"),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&types::ConnectorAuthType> for PlacetopayAuth {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        let placetopay_auth = PlacetopayAuthType::try_from(auth_type)?;
        let nonce_bytes = utils::generate_random_bytes(16);
        let now = date_time::date_as_yyyymmddthhmmssmmmz()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let seed = format!("{}+00:00", now.split_at(now.len() - 5).0);
        let mut context = digest::Context::new(&digest::SHA256);
        context.update(&nonce_bytes);
        context.update(seed.as_bytes());
        context.update(placetopay_auth.tran_key.peek().as_bytes());
        let encoded_digest = base64::Engine::encode(&consts::BASE64_ENGINE, context.finish());
        let nonce = Secret::new(base64::Engine::encode(&consts::BASE64_ENGINE, &nonce_bytes));

        println!("$$seed: {}", seed);
        println!("$$nonce: {}", nonce.peek());
        println!("$$login: {}", placetopay_auth.login.peek());
        println!("$$tran_key: {}", placetopay_auth.tran_key.peek());
        Ok(Self {
            login: placetopay_auth.login,
            tran_key: encoded_digest.into(),
            nonce,
            seed,
        })
    }
}

impl TryFrom<&types::ConnectorAuthType> for PlacetopayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                login: api_key.to_owned(),
                tran_key: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlacetopayTransactionStatus {
    Ok,
    Failed,
    Approved,
    // ApprovedPartial,
    // PartialExpired,
    Rejected,
    Pending,
    PendingValidation,
    PendingProcess,
    // Refunded,
    // Reversed,
    Error,
    // Unknown,
    // Manual,
    // Dispute,
    //The statuses that are commented out are awaiting clarification on the connector.
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayStatusResponse {
    status: PlacetopayTransactionStatus,
}

impl From<PlacetopayTransactionStatus> for enums::AttemptStatus {
    fn from(item: PlacetopayTransactionStatus) -> Self {
        match item {
            PlacetopayTransactionStatus::Approved => Self::Charged,

            PlacetopayTransactionStatus::Ok => Self::AuthenticationPending,
            PlacetopayTransactionStatus::Failed
            | PlacetopayTransactionStatus::Rejected
            | PlacetopayTransactionStatus::Error => Self::Failure,
            PlacetopayTransactionStatus::Pending
            | PlacetopayTransactionStatus::PendingValidation
            | PlacetopayTransactionStatus::PendingProcess => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlacetopayCompleteAuthorizeStatus {
    Ok,
    Failed,
    Approved,
    // ApprovedPartial,
    // PartialExpired,
    Rejected,
    Pending,
    PendingValidation,
    PendingProcess,
    // Refunded,
    // Reversed,
    Error,
    // Unknown,
    // Manual,
    // Dispute,
    //The statuses that are commented out are awaiting clarification on the connector.
}

impl From<PlacetopayCompleteAuthorizeStatus> for enums::AttemptStatus {
    fn from(item: PlacetopayCompleteAuthorizeStatus) -> Self {
        match item {
            PlacetopayCompleteAuthorizeStatus::Approved | PlacetopayCompleteAuthorizeStatus::Ok => {
                Self::Charged
            }
            PlacetopayCompleteAuthorizeStatus::Failed
            | PlacetopayCompleteAuthorizeStatus::Rejected
            | PlacetopayCompleteAuthorizeStatus::Error => Self::Failure,
            PlacetopayCompleteAuthorizeStatus::Pending
            | PlacetopayCompleteAuthorizeStatus::PendingValidation
            | PlacetopayCompleteAuthorizeStatus::PendingProcess => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PlacetopayPaymentsResponse {
    PlacetopayNo3dsResponse(PlacetopayNo3dsResponse),
    Placetopay3dsResponse(Placetopay3dsResponse),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayNo3dsResponse {
    pub status: PlacetopayStatusResponse,
    pub internal_reference: u64,
    pub authorization: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placetopay3dsResponse {
    pub status: PlacetopayStatusResponse,
    pub data: Placetopay3dsData,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placetopay3dsData {
    redirect_url: String,
    identifier: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayCompleteAuthorizeResponse {
    pub status: Placetopay3dsStatusResponse,
    pub data: PlacetopayPsync3dsData,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayPsync3dsData {
    id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placetopay3dsStatusResponse {
    status: PlacetopayCompleteAuthorizeStatus,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            PlacetopayCompleteAuthorizeResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PlacetopayCompleteAuthorizeResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, PlacetopayPaymentsResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PlacetopayPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            PlacetopayPaymentsResponse::Placetopay3dsResponse(response) => {
                let url = response.data.redirect_url;
                println!("$$url: {}", url);
                let redirection_data = Some(services::RedirectForm::Form {
                    endpoint: url,
                    method: Method::Get,
                    form_fields: Default::default(),
                });
                println!("$$redirection_data: {:?}", redirection_data);
                Ok(Self {
                    status: enums::AttemptStatus::from(response.status.status),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            response.data.identifier.to_string(),
                        ),
                        redirection_data: Box::new(redirection_data),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
            PlacetopayPaymentsResponse::PlacetopayNo3dsResponse(response) => Ok(Self {
                status: enums::AttemptStatus::from(response.status.status),
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        response.internal_reference.to_string(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: response
                        .authorization
                        .clone()
                        .map(|authorization| serde_json::json!(authorization)),
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
        }
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayRefundRequest {
    auth: PlacetopayAuth,
    internal_reference: u64,
    action: PlacetopayNextAction,
    authorization: Option<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for PlacetopayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        if item.request.minor_refund_amount == item.request.minor_payment_amount {
            let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;

            let internal_reference = item
                .request
                .connector_transaction_id
                .parse::<u64>()
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            let action = PlacetopayNextAction::Reverse;
            let authorization = match item.request.connector_metadata.clone() {
                Some(metadata) => metadata.as_str().map(|auth| auth.to_string()),
                None => None,
            };
            Ok(Self {
                auth,
                internal_reference,
                action,
                authorization,
            })
        } else {
            Err(errors::ConnectorError::NotSupported {
                message: "Partial Refund".to_string(),
                connector: "placetopay",
            }
            .into())
        }
    }
}

impl From<PlacetopayRefundStatus> for enums::RefundStatus {
    fn from(item: PlacetopayRefundStatus) -> Self {
        match item {
            PlacetopayRefundStatus::Ok
            | PlacetopayRefundStatus::Approved
            | PlacetopayRefundStatus::Refunded => Self::Success,
            PlacetopayRefundStatus::Failed
            | PlacetopayRefundStatus::Rejected
            | PlacetopayRefundStatus::Error => Self::Failure,
            PlacetopayRefundStatus::Pending
            | PlacetopayRefundStatus::PendingProcess
            | PlacetopayRefundStatus::PendingValidation => Self::Pending,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlacetopayRefundStatus {
    Ok,
    Failed,
    Approved,
    // ApprovedPartial,
    // PartialExpired,
    Rejected,
    Pending,
    PendingValidation,
    PendingProcess,
    Refunded,
    // Reversed,
    Error,
    // Unknown,
    // Manual,
    // Dispute,
    //The statuses that are commented out are awaiting clarification on the connector.
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayRefundStatusResponse {
    status: PlacetopayRefundStatus,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayRefundResponse {
    status: PlacetopayRefundStatusResponse,
    internal_reference: u64,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, PlacetopayRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, PlacetopayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.internal_reference.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayRsyncRequest {
    auth: PlacetopayAuth,
    internal_reference: u64,
}

impl TryFrom<&types::RefundsRouterData<api::RSync>> for PlacetopayRsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<api::RSync>) -> Result<Self, Self::Error> {
        let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;
        let internal_reference = item
            .request
            .connector_transaction_id
            .parse::<u64>()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            auth,
            internal_reference,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, PlacetopayRefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, PlacetopayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.internal_reference.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayErrorResponse {
    pub status: PlacetopayError,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayError {
    pub status: PlacetopayErrorStatus,
    pub message: String,
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlacetopayErrorStatus {
    Failed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayPsyncRequest {
    auth: PlacetopayAuth,
    internal_reference: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayCompleteAuthorizeRequest {
    auth: PlacetopayAuth,
    id: String,
    instrument: PlacetopayInstrument,
}

impl TryFrom<&types::PaymentsSyncRouterData> for PlacetopayPsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;
        let internal_reference = item
            .request
            .get_connector_transaction_id()?
            .parse::<u64>()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            auth,
            internal_reference,
        })
    }
}

impl TryFrom<&types::PaymentsCompleteAuthorizeRouterData> for PlacetopayCompleteAuthorizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;
        let id = item.request.connector_transaction_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_transaction_id",
            },
        )?;

        match item.request.payment_method_data.clone() {
            Some(domain::PaymentMethodData::Card(req_card)) => {
                println!("$$req_card: {:?}", req_card);
                let card = PlacetopayCard {
                    number: req_card.card_number.clone(),
                    expiration: req_card
                        .clone()
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned())?,
                    cvv: req_card.card_cvc.clone(),
                };
                Ok(Self {
                    auth,
                    id,
                    instrument: PlacetopayInstrument {
                        card: card.to_owned(),
                    },
                })
            }
            Some(domain::PaymentMethodData::BankTransfer(..))
            | Some(domain::PaymentMethodData::Wallet(..))
            | Some(domain::PaymentMethodData::BankDebit(..))
            | Some(domain::PaymentMethodData::BankRedirect(..))
            | Some(domain::PaymentMethodData::PayLater(..))
            | Some(domain::PaymentMethodData::Crypto(..))
            | Some(domain::PaymentMethodData::Reward)
            | Some(domain::PaymentMethodData::RealTimePayment(..))
            | Some(domain::PaymentMethodData::MobilePayment(..))
            | Some(domain::PaymentMethodData::MandatePayment)
            | Some(domain::PaymentMethodData::Upi(..))
            | Some(domain::PaymentMethodData::GiftCard(..))
            | Some(domain::PaymentMethodData::CardRedirect(..))
            | Some(domain::PaymentMethodData::Voucher(..))
            | Some(domain::PaymentMethodData::OpenBanking(..))
            | Some(domain::PaymentMethodData::CardToken(..))
            | Some(domain::PaymentMethodData::NetworkToken(..))
            | Some(domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_))
            | None => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Placetopay"),
            )
            .into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayNextActionRequest {
    auth: PlacetopayAuth,
    internal_reference: u64,
    action: PlacetopayNextAction,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacetopayNextAction {
    Refund,
    Reverse,
    Void,
    Process,
    Checkout,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for PlacetopayNextActionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;
        let internal_reference = item
            .request
            .connector_transaction_id
            .parse::<u64>()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let action = PlacetopayNextAction::Checkout;
        Ok(Self {
            auth,
            internal_reference,
            action,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for PlacetopayNextActionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;
        let internal_reference = item
            .request
            .connector_transaction_id
            .parse::<u64>()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let action = PlacetopayNextAction::Void;
        Ok(Self {
            auth,
            internal_reference,
            action,
        })
    }
}
