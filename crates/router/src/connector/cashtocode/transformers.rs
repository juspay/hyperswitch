use common_utils::pii::Email;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    services,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodePaymentsRequest {
    amount: f64,
    transaction_id: String,
    user_id: Secret<String>,
    currency: enums::Currency,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    user_alias: Secret<String>,
    requested_url: String,
    cancel_url: String,
    email: Option<Email>,
    mid: String,
}

pub struct CashToCodeMandatoryParams {
    pub user_id: Secret<String>,
    pub user_alias: Secret<String>,
    pub requested_url: String,
    pub cancel_url: String,
}

fn get_mid(
    payment_method_data: &api::payments::PaymentMethodData,
) -> Result<String, errors::ConnectorError> {
    match payment_method_data {
        api_models::payments::PaymentMethodData::Reward(reward_data) => {
            Ok(reward_data.merchant_id.to_string())
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment methods".to_string(),
        )),
    }
}

fn get_mandatory_params(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<CashToCodeMandatoryParams, error_stack::Report<errors::ConnectorError>> {
    let customer_id = item.get_customer_id()?;
    let url = item.request.get_router_return_url()?;
    Ok(CashToCodeMandatoryParams {
        user_id: Secret::new(customer_id.to_owned()),
        user_alias: Secret::new(customer_id),
        requested_url: url.to_owned(),
        cancel_url: url,
    })
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CashtocodePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let params: CashToCodeMandatoryParams = get_mandatory_params(item)?;
        let mid = get_mid(&item.request.payment_method_data)?;
        match item.payment_method {
            diesel_models::enums::PaymentMethod::Reward => Ok(Self {
                amount: utils::to_currency_base_unit_asf64(
                    item.request.amount,
                    item.request.currency,
                )?,
                transaction_id: item.attempt_id.clone(),
                currency: item.request.currency,
                user_id: params.user_id,
                first_name: None,
                last_name: None,
                user_alias: params.user_alias,
                requested_url: params.requested_url,
                cancel_url: params.cancel_url,
                email: item.request.email.clone(),
                mid,
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct CashtocodeAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for CashtocodeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, .. } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CashtocodePaymentStatus {
    Succeeded,
    #[default]
    Processing,
}

impl From<CashtocodePaymentStatus> for enums::AttemptStatus {
    fn from(item: CashtocodePaymentStatus) -> Self {
        match item {
            CashtocodePaymentStatus::Succeeded => Self::Charged,
            CashtocodePaymentStatus::Processing => Self::AuthenticationPending,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CashtocodeErrors {
    pub message: String,
    pub path: String,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CashtocodePaymentsResponse {
    CashtoCodeError(CashtocodeErrorResponse),
    CashtoCodeData(CashtocodePaymentsResponseData),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodePaymentsResponseData {
    pub pay_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodePaymentsSyncResponse {
    pub transaction_id: String,
    pub amount: i64,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, CashtocodePaymentsResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CashtocodePaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response {
            CashtocodePaymentsResponse::CashtoCodeError(error_data) => (
                enums::AttemptStatus::Failure,
                Err(types::ErrorResponse {
                    code: error_data.error.to_string(),
                    status_code: item.http_code,
                    message: error_data.error_description,
                    reason: None,
                }),
            ),
            CashtocodePaymentsResponse::CashtoCodeData(response_data) => {
                let redirection_data = services::RedirectForm::Form {
                    endpoint: response_data.pay_url,
                    method: services::Method::Post,
                    form_fields: Default::default(),
                };
                (
                    enums::AttemptStatus::AuthenticationPending,
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            item.data.attempt_id.clone(),
                        ),
                        redirection_data: Some(redirection_data),
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                    }),
                )
            }
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            CashtocodePaymentsSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CashtocodePaymentsSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Charged,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.data.attempt_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            amount_captured: Some(item.response.amount),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CashtocodeErrorResponse {
    pub error: u32,
    pub error_description: String,
    pub errors: Option<Vec<CashtocodeErrors>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodeIncomingWebhook {
    pub amount: i64,
    pub currency: String,
    pub foreign_transaction_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub transaction_id: String,
}
