use common_utils::pii::Email;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    services,
    types::{self, storage::enums},
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
    mid: Secret<String>,
}

pub struct CashToCodeMandatoryParams {
    pub user_id: Secret<String>,
    pub user_alias: Secret<String>,
    pub requested_url: String,
    pub cancel_url: String,
}

fn get_mid(
    connector_auth_type: &types::ConnectorAuthType,
    payment_method_type: Option<enums::PaymentMethodType>,
) -> Result<Secret<String>, errors::ConnectorError> {
    match (connector_auth_type, payment_method_type) {
        (types::ConnectorAuthType::MultiAuthKey { key1, key2, .. }, Some(payment_method)) => {
            match payment_method {
                enums::PaymentMethodType::ClassicReward => Ok(key1.to_owned()),
                enums::PaymentMethodType::Evoucher => Ok(key2.to_owned()),
                _ => Err(errors::ConnectorError::NotSupported {
                    message: payment_method.to_string(),
                    connector: "cashtocode",
                    payment_experience: "Try with a different payment method".to_string(),
                }),
            }
        }
        (_, None) => Err(errors::ConnectorError::MissingPaymentMethodType),
        _ => Err(errors::ConnectorError::FailedToObtainAuthType),
    }
}

fn get_mandatory_params(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<CashToCodeMandatoryParams, error_stack::Report<errors::ConnectorError>> {
    let customer_id = item.get_customer_id()?;
    let return_url = item.request.get_router_return_url()?;

    Ok(CashToCodeMandatoryParams {
        user_id: Secret::new(customer_id.to_owned()),
        user_alias: Secret::new(customer_id),
        requested_url: return_url.to_owned(),
        cancel_url: return_url,
    })
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CashtocodePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let params: CashToCodeMandatoryParams = get_mandatory_params(item)?;
        let mid = get_mid(&item.connector_auth_type, item.request.payment_method_type)?;
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
    pub(super) api_key_classic: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for CashtocodeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::MultiAuthKey { api_key, .. } => Ok(Self {
                api_key_classic: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
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
    pub error: String,
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
