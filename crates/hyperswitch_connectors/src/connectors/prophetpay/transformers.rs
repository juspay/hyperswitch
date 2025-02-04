use std::collections::HashMap;

use common_enums::enums;
use common_utils::{
    consts::{PROPHETPAY_REDIRECT_URL, PROPHETPAY_TOKEN},
    errors::CustomResult,
    request::Method,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{CardRedirectData, PaymentMethodData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::{
        CompleteAuthorizeData, CompleteAuthorizeRedirectResponse, PaymentsAuthorizeData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::{api, consts::NO_ERROR_CODE, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, to_connector_meta},
};

pub struct ProphetpayRouterData<T> {
    pub amount: f64,
    pub router_data: T,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for ProphetpayRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
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

impl TryFrom<&ConnectorAuthType> for ProphetpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
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
                PaymentMethodData::CardRedirect(CardRedirectData::CardRedirect {}) => {
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
            Err(errors::ConnectorError::CurrencyNotSupported {
                message: item.router_data.request.currency.to_string(),
                connector: "Prophetpay",
            }
            .into())
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayTokenResponse {
    hosted_tokenize_id: Secret<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, ProphetpayTokenResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            ProphetpayTokenResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let url_data = format!(
            "{}{}",
            PROPHETPAY_REDIRECT_URL,
            item.response.hosted_tokenize_id.expose()
        );

        let redirect_url = Url::parse(url_data.as_str())
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let redirection_data = get_redirect_url_form(
            redirect_url,
            item.data.request.complete_authorize_url.clone(),
        )
        .ok();

        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

fn get_redirect_url_form(
    mut redirect_url: Url,
    complete_auth_url: Option<String>,
) -> CustomResult<RedirectForm, errors::ConnectorError> {
    let mut form_fields = HashMap::<String, String>::new();

    form_fields.insert(
        String::from("redirectUrl"),
        complete_auth_url.ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "complete_auth_url",
        })?,
    );

    // Do not include query params in the endpoint
    redirect_url.set_query(None);

    Ok(RedirectForm::Form {
        endpoint: redirect_url.to_string(),
        method: Method::Get,
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
    card_token: Secret<String>,
}

impl TryFrom<&ProphetpayRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for ProphetpayCompleteRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ProphetpayRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_data = ProphetpayAuthType::try_from(&item.router_data.connector_auth_type)?;
        let card_token = Secret::new(get_card_token(
            item.router_data.request.redirect_response.clone(),
        )?);
        Ok(Self {
            amount: item.amount.to_owned(),
            ref_info: item.router_data.connector_request_reference_id.to_owned(),
            inquiry_reference: item.router_data.connector_request_reference_id.clone(),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Charge),
            card_token,
        })
    }
}

fn get_card_token(
    response: Option<CompleteAuthorizeRedirectResponse>,
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
                queries.insert(
                    pair.first()
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?
                        .to_string(),
                    pair.get(1)
                        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?
                        .to_string(),
                );
            }
            Ok::<_, errors::ConnectorError>(queries)
        })
        .transpose()?
        .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

    for (key, val) in queries_params {
        if key.as_str() == PROPHETPAY_TOKEN {
            return Ok(val);
        }
    }

    Err(errors::ConnectorError::MissingRequiredField {
        field_name: "card_token",
    }
    .into())
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
            ref_info: item.connector_request_reference_id.to_owned(),
            inquiry_reference: item.connector_request_reference_id.clone(),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Inquiry),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayCompleteAuthResponse {
    pub success: bool,
    pub response_text: String,
    #[serde(rename = "transactionID")]
    pub transaction_id: String,
    pub response_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProphetpayCardTokenData {
    card_token: Secret<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            ProphetpayCompleteAuthResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            ProphetpayCompleteAuthResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        if item.response.success {
            let card_token = get_card_token(item.data.request.redirect_response.clone())?;
            let card_token_data = ProphetpayCardTokenData {
                card_token: Secret::from(card_token),
            };
            let connector_metadata = serde_json::to_value(card_token_data).ok();
            Ok(Self {
                status: enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.response_text.clone(),
                    reason: Some(item.response.response_text),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpaySyncResponse {
    success: bool,
    pub response_text: String,
    #[serde(rename = "transactionID")]
    pub transaction_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, ProphetpaySyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ProphetpaySyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if item.response.success {
            Ok(Self {
                status: enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: NO_ERROR_CODE.to_string(),
                    message: item.response.response_text.clone(),
                    reason: Some(item.response.response_text),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayVoidResponse {
    pub success: bool,
    pub response_text: String,
    #[serde(rename = "transactionID")]
    pub transaction_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, ProphetpayVoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ProphetpayVoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if item.response.success {
            Ok(Self {
                status: enums::AttemptStatus::Voided,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::VoidFailed,
                response: Err(ErrorResponse {
                    code: NO_ERROR_CODE.to_string(),
                    message: item.response.response_text.clone(),
                    reason: Some(item.response.response_text),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
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
            ref_info: item.connector_request_reference_id.to_owned(),
            inquiry_reference: item.connector_request_reference_id.clone(),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Inquiry),
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayRefundRequest {
    pub amount: f64,
    pub card_token: Secret<String>,
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
        if item.router_data.request.payment_amount == item.router_data.request.refund_amount {
            let auth_data = ProphetpayAuthType::try_from(&item.router_data.connector_auth_type)?;
            let transaction_id = item.router_data.request.connector_transaction_id.to_owned();
            let card_token_data: ProphetpayCardTokenData =
                to_connector_meta(item.router_data.request.connector_metadata.clone())?;

            Ok(Self {
                transaction_id,
                amount: item.amount.to_owned(),
                card_token: card_token_data.card_token,
                profile: auth_data.profile_id,
                ref_info: item.router_data.request.refund_id.to_owned(),
                inquiry_reference: item.router_data.request.refund_id.clone(),
                action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Refund),
            })
        } else {
            Err(errors::ConnectorError::NotImplemented("Partial Refund".to_string()).into())
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayRefundResponse {
    pub success: bool,
    pub response_text: String,
    pub tran_seq_number: Option<String>,
}

impl TryFrom<RefundsResponseRouterData<Execute, ProphetpayRefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, ProphetpayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        if item.response.success {
            Ok(Self {
                response: Ok(RefundsResponseData {
                    // no refund id is generated, tranSeqNumber is kept for future usage
                    connector_refund_id: item.response.tran_seq_number.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "tran_seq_number",
                        },
                    )?,
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: NO_ERROR_CODE.to_string(),
                    message: item.response.response_text.clone(),
                    reason: Some(item.response.response_text),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayRefundSyncResponse {
    pub success: bool,
    pub response_text: String,
}

impl<T> TryFrom<RefundsResponseRouterData<T, ProphetpayRefundSyncResponse>>
    for types::RefundsRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<T, ProphetpayRefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        if item.response.success {
            Ok(Self {
                response: Ok(RefundsResponseData {
                    // no refund id is generated, rather transaction id is used for referring to status in refund also
                    connector_refund_id: item.data.request.connector_transaction_id.clone(),
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            })
        } else {
            Ok(Self {
                status: enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: NO_ERROR_CODE.to_string(),
                    message: item.response.response_text.clone(),
                    reason: Some(item.response.response_text),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
                ..item.data
            })
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProphetpayRefundSyncRequest {
    transaction_id: String,
    inquiry_reference: String,
    ref_info: String,
    profile: Secret<String>,
    action_type: i8,
}

impl TryFrom<&types::RefundSyncRouterData> for ProphetpayRefundSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth_data = ProphetpayAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            transaction_id: item.request.connector_transaction_id.clone(),
            ref_info: item.connector_request_reference_id.to_owned(),
            inquiry_reference: item.connector_request_reference_id.clone(),
            profile: auth_data.profile_id,
            action_type: ProphetpayActionType::get_action_type(&ProphetpayActionType::Inquiry),
        })
    }
}
