use api_models::payments;
use common_utils::date_time;
use diesel_models::enums;
use error_stack::{IntoReport, ResultExt};
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
    types::{self, api, storage::enums as storage_enums},
};

pub struct PlacetopayRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PlacetopayRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
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
    nonce: String,
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
    total: i64,
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
            payments::PaymentMethodData::Card(req_card) => {
                let card = PlacetopayCard {
                    number: req_card.card_number.clone(),
                    expiration: req_card
                        .clone()
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned())?,
                    cvv: req_card.card_cvc.clone(),
                };
                Ok(Self {
                    ip_address,
                    user_agent,
                    auth,
                    payment,
                    instrument: PlacetopayInstrument {
                        card: card.to_owned(),
                    },
                })
            }
            payments::PaymentMethodData::Wallet(_)
            | payments::PaymentMethodData::CardRedirect(_)
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
        let now = error_stack::IntoReport::into_report(date_time::date_as_yyyymmddthhmmssmmmz())
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let seed = format!("{}+00:00", now.split_at(now.len() - 5).0);
        let mut context = digest::Context::new(&digest::SHA256);
        context.update(&nonce_bytes);
        context.update(seed.as_bytes());
        context.update(placetopay_auth.tran_key.peek().as_bytes());
        let encoded_digest = base64::Engine::encode(&consts::BASE64_ENGINE, context.finish());
        let nonce = base64::Engine::encode(&consts::BASE64_ENGINE, &nonce_bytes);
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
pub enum PlacetopayStatus {
    Ok,
    Failed,
    Approved,
    Rejected,
    Pending,
    PendingValidation,
    PendingProcess,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayStatusResponse {
    status: PlacetopayStatus,
}

impl From<PlacetopayStatus> for enums::AttemptStatus {
    fn from(item: PlacetopayStatus) -> Self {
        match item {
            PlacetopayStatus::Approved | PlacetopayStatus::Ok => Self::Authorized,
            PlacetopayStatus::Failed | PlacetopayStatus::Rejected => Self::Failure,
            PlacetopayStatus::Pending
            | PlacetopayStatus::PendingValidation
            | PlacetopayStatus::PendingProcess => Self::Authorizing,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayPaymentsResponse {
    status: PlacetopayStatusResponse,
    internal_reference: u64,
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
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.internal_reference.to_string(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
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
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for PlacetopayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;
        let internal_reference = item
            .request
            .connector_transaction_id
            .parse::<u64>()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let action = PlacetopayNextAction::Refund;

        Ok(Self {
            auth,
            internal_reference,
            action,
        })
    }
}

impl From<PlacetopayRefundStatus> for enums::RefundStatus {
    fn from(item: PlacetopayRefundStatus) -> Self {
        match item {
            PlacetopayRefundStatus::Refunded => Self::Success,
            PlacetopayRefundStatus::Failed | PlacetopayRefundStatus::Rejected => Self::Failure,
            PlacetopayRefundStatus::Pending | PlacetopayRefundStatus::PendingProcess => {
                Self::Pending
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacetopayRefundResponse {
    status: PlacetopayRefundStatus,
    internal_reference: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlacetopayRefundStatus {
    Refunded,
    Rejected,
    Failed,
    Pending,
    PendingProcess,
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
                refund_status: enums::RefundStatus::from(item.response.status),
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
            .into_report()
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
                refund_status: enums::RefundStatus::from(item.response.status),
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

impl TryFrom<&types::PaymentsSyncRouterData> for PlacetopayPsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = PlacetopayAuth::try_from(&item.connector_auth_type)?;
        let internal_reference = item
            .request
            .get_connector_transaction_id()?
            .parse::<u64>()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            auth,
            internal_reference,
        })
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
            .into_report()
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
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let action = PlacetopayNextAction::Void;
        Ok(Self {
            auth,
            internal_reference,
            action,
        })
    }
}
