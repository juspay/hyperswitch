use std::collections::HashMap;

use common_utils::{consts, errors::CustomResult};
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils,
    core::errors,
    services,
    types::{self, api, storage::enums},
};

pub struct ProphetpayRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for ProphetpayRouterData<T>
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
        let amount = utils::get_amount_as_f64(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

pub struct ProphetpayAuthType {
    pub(super) user_name: Secret<String>,
    pub(super) password: Secret<String>,
    pub(super) profile_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for ProphetpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                user_name: api_key.to_owned(),
                password: key1.to_owned(),
                profile_id: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProphetpayTokenRequest {
    ref_info: String,
    profile: Secret<String>,
    entry_method: i8,
    token_type: i8,
    card_entry_context: i8,
}

#[derive(Debug, Clone)]
pub enum ProphetpayEntryMethod {
    ManualEntry,
    CardSwipe,
}

impl ProphetpayEntryMethod {
    fn get_entry_method(&self) -> i8 {
        match self {
            Self::ManualEntry => 1,
            Self::CardSwipe => 2,
        }
    }
}

#[derive(Debug, Clone)]
#[repr(i8)]
pub enum ProphetpayTokenType {
    Normal,
    SaleTab,
    TemporarySave,
}

impl ProphetpayTokenType {
    fn get_token_type(&self) -> i8 {
        match self {
            Self::Normal => 0,
            Self::SaleTab => 1,
            Self::TemporarySave => 2,
        }
    }
}

#[derive(Debug, Clone)]
#[repr(i8)]
pub enum ProphetpayCardContext {
    NotApplicable,
    WebConsumerInitiated,
}

impl ProphetpayCardContext {
    fn get_card_context(&self) -> i8 {
        match self {
            Self::NotApplicable => 0,
            Self::WebConsumerInitiated => 5,
        }
    }
}

impl TryFrom<&ProphetpayRouterData<&types::PaymentsAuthorizeRouterData>>
    for ProphetpayTokenRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ProphetpayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.request.currency == api_models::enums::Currency::USD {
            match item.router_data.request.payment_method_data.clone() {
                api::PaymentMethodData::CardRedirect(
                    api_models::payments::CardRedirectData::CardRedirect {},
                ) => {
                    let auth_data =
                        ProphetpayAuthType::try_from(&item.router_data.connector_auth_type)?;
                    Ok(Self {
                        ref_info: item.router_data.connector_request_reference_id.to_owned(),
                        profile: auth_data.profile_id,
                        entry_method: ProphetpayEntryMethod::get_entry_method(
                            &ProphetpayEntryMethod::ManualEntry,
                        ),
                        token_type: ProphetpayTokenType::get_token_type(
                            &ProphetpayTokenType::SaleTab,
                        ),
                        card_entry_context: ProphetpayCardContext::get_card_context(
                            &ProphetpayCardContext::WebConsumerInitiated,
                        ),
                    })
                }
                _ => Err(
                    errors::ConnectorError::NotImplemented("Payment methods".to_string()).into(),
                ),
            }
        } else {
            Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayTokenResponse {
    hosted_tokenize_id: String,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            ProphetpayTokenResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            ProphetpayTokenResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let url_data = format!(
            "{}{}",
            consts::PROPHETPAY_REDIRECT_URL,
            item.response.hosted_tokenize_id
        );

        let redirect_url = Url::parse(url_data.as_str())
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let redirection_data = get_redirect_url_form(
            redirect_url,
            item.data.request.complete_authorize_url.clone(),
        )
        .ok();

        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::NoResponseId,
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

fn get_redirect_url_form(
    mut redirect_url: Url,
    complete_auth_url: Option<String>,
) -> CustomResult<services::RedirectForm, errors::ConnectorError> {
    let mut form_fields = std::collections::HashMap::<String, String>::new();

    form_fields.insert(
        String::from("redirectUrl"),
        complete_auth_url.ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "complete_auth_url",
        })?,
    );

    // Do not include query params in the endpoint
    redirect_url.set_query(None);

    Ok(services::RedirectForm::Form {
        endpoint: redirect_url.to_string(),
        method: services::Method::Get,
        form_fields,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayCompleteRequest {
    amount: f64,
    ref_info: String,
    inquiry_reference: String,
    profile: Secret<String>,
    action_type: i8,
    card_token: String,
}

impl TryFrom<&ProphetpayRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for ProphetpayCompleteRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ProphetpayRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_data = ProphetpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        let card_token = get_card_token(item.router_data.request.redirect_response.clone())?;
        Ok(Self {
            amount: item.amount.to_owned(),
            ref_info: item.router_data.connector_request_reference_id.to_owned(),
            inquiry_reference: format!(
                "inquiry_{}",
                item.router_data.connector_request_reference_id
            ),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Charge),
            card_token,
        })
    }
}

fn get_card_token(
    response: Option<types::CompleteAuthorizeRedirectResponse>,
) -> CustomResult<String, errors::ConnectorError> {
    let res = response.ok_or(errors::ConnectorError::MissingRequiredField {
        field_name: "redirect_response",
    })?;
    let queries_params = res
        .params
        .map(|param| {
            let mut queries = HashMap::<String, String>::new();
            let values = param.peek().split('&').collect::<Vec<&str>>();
            for value in values {
                let pair = value.split('=').collect::<Vec<&str>>();
                queries.insert(pair[0].to_string(), pair[1].to_string());
            }
            queries
        })
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

    for (key, val) in queries_params {
        if key.as_str() == consts::PROPHETPAY_TOKEN {
            return Ok(val);
        }
    }

    Err(errors::ConnectorError::MissingRequiredField {
        field_name: "card_token",
    })
    .into_report()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpaySyncRequest {
    transaction_id: String,
    ref_info: String,
    inquiry_reference: String,
    profile: Secret<String>,
    action_type: i8,
}

#[derive(Debug, Clone)]
pub enum ProphetpayActionType {
    Charge,
    Refund,
    Inquiry,
}

impl ProphetpayActionType {
    fn get_action_type(&self) -> i8 {
        match self {
            Self::Charge => 1,
            Self::Refund => 3,
            Self::Inquiry => 7,
        }
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for ProphetpaySyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth_data = ProphetpayAuthType::try_from(&item.connector_auth_type)?;
        let transaction_id = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        Ok(Self {
            transaction_id,
            ref_info: item.attempt_id.to_owned(),
            inquiry_reference: format!("inquiry_{}", item.attempt_id),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Inquiry),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum ProphetpayPaymentStatus {
    Success,
    #[serde(rename = "Transaction Approved")]
    Charged,
    Failure,
    #[serde(rename = "Transaction Voided")]
    Voided,
    #[serde(rename = "Requires a card on file.")]
    CardTokenNotFound,
    #[serde(rename = "RefInfo and InquiryReference are duplicated")]
    DuplicateValue,
    #[serde(rename = "Profile is missing")]
    MissingProfile,
    #[serde(rename = "RefInfo is empty.")]
    EmptyRef,
}

impl From<ProphetpayPaymentStatus> for enums::AttemptStatus {
    fn from(item: ProphetpayPaymentStatus) -> Self {
        match item {
            ProphetpayPaymentStatus::Success | ProphetpayPaymentStatus::Charged => Self::Charged,
            ProphetpayPaymentStatus::Failure
            | ProphetpayPaymentStatus::CardTokenNotFound
            | ProphetpayPaymentStatus::DuplicateValue
            | ProphetpayPaymentStatus::MissingProfile
            | ProphetpayPaymentStatus::EmptyRef => Self::Failure,
            ProphetpayPaymentStatus::Voided => Self::Voided,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayResponse {
    pub response_text: ProphetpayPaymentStatus,
    #[serde(rename = "transactionID")]
    pub transaction_id: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, ProphetpayResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ProphetpayResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.response_text),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayVoidRequest {
    pub transaction_id: String,
    pub profile: Secret<String>,
    pub ref_info: String,
    pub inquiry_reference: String,
    pub action_type: i8,
}

impl TryFrom<&types::PaymentsCancelRouterData> for ProphetpayVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth_data = ProphetpayAuthType::try_from(&item.connector_auth_type)?;
        let transaction_id = item.request.connector_transaction_id.to_owned();
        Ok(Self {
            transaction_id,
            ref_info: item.attempt_id.to_owned(),
            inquiry_reference: format!("inquiry_{}", item.attempt_id),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Inquiry),
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayRefundRequest {
    pub amount: f64,
    pub transaction_id: String,
    pub profile: Secret<String>,
    pub ref_info: String,
    pub inquiry_reference: String,
    pub action_type: i8,
}

impl<F> TryFrom<&ProphetpayRouterData<&types::RefundsRouterData<F>>> for ProphetpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ProphetpayRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let auth_data = ProphetpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        let transaction_id = item.router_data.request.connector_transaction_id.to_owned();
        Ok(Self {
            transaction_id,
            amount: item.amount.to_owned(),
            profile: auth_data.profile_id,
            ref_info: item.router_data.request.refund_id.to_owned(),
            inquiry_reference: format!("inquiry_{}", item.router_data.request.refund_id),
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Refund),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub enum RefundStatus {
    Success,
    Failure,
    #[serde(rename = "Transaction Voided")]
    Voided,
    #[serde(rename = "Requires a card on file.")]
    CardTokenNotFound,
    #[serde(rename = "RefInfo and InquiryReference are duplicated")]
    DuplicateValue,
    #[serde(rename = "Profile is missing")]
    MissingProfile,
    #[serde(rename = "RefInfo is empty.")]
    EmptyRef,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success
            // in retrieving refund, if it is successful, it is shown as voided
            | RefundStatus::Voided => Self::Success,
            RefundStatus::Failure
            | RefundStatus::CardTokenNotFound
            | RefundStatus::DuplicateValue
            | RefundStatus::MissingProfile
            | RefundStatus::EmptyRef => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayRefundResponse {
    pub response_text: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, ProphetpayRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, ProphetpayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                // no refund id is generated, rather transaction id is used for referring to status in refund also
                connector_refund_id: item.data.request.connector_transaction_id.clone(),
                refund_status: enums::RefundStatus::from(item.response.response_text),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayRefundSyncRequest {
    ref_info: String,
    profile: Secret<String>,
    action_type: i8,
}

impl TryFrom<&types::RefundSyncRouterData> for ProphetpayRefundSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth_data = ProphetpayAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            ref_info: item.attempt_id.to_owned(),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Inquiry),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, ProphetpayRefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, ProphetpayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.data.request.connector_transaction_id.clone(),
                refund_status: enums::RefundStatus::from(item.response.response_text),
            }),
            ..item.data
        })
    }
}

// Error Response body is yet to be confirmed with the connector
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayErrorResponse {
    pub status: u16,
    pub title: String,
    pub trace_id: String,
    pub errors: serde_json::Value,
}
