use std::collections::HashMap;

use common_enums::AttemptStatus;
use common_utils::request::Method;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::{PaymentsAuthenticateData, PaymentsPostAuthenticateData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::ResponseRouterData;

pub struct BiopayAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BiopayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BiopayAuthenticateRequest {
    pub platform: String,
    pub platform_merchant_id: String,
    pub platform_profile_id: Option<String>,
    pub platform_payment_id: String,
    pub amount: String,
    pub currency: String,
    pub return_url: String,
    pub metadata: serde_json::Value,
}

impl TryFrom<&RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>>
    for BiopayAuthenticateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RouterData<api::Authenticate, PaymentsAuthenticateData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let amount_minor = item.request.amount.unwrap_or(0);
        let units = amount_minor / 100;
        let cents = amount_minor % 100;
        let amount = format!("{}.{:02}", units, cents);

        let currency = item
            .request
            .currency
            .map(|currency| currency.to_string())
            .unwrap_or_else(|| "USD".to_string());

        let return_url = item
            .request
            .complete_authorize_url
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "complete_authorize_url",
            })?;

        Ok(Self {
            platform: "hyperswitch".to_string(),
            platform_merchant_id: item.merchant_id.to_string(),
            platform_profile_id: None,
            platform_payment_id: item.payment_id.to_string(),
            amount,
            currency,
            return_url,
            metadata: serde_json::json!({
                "attempt_id": item.attempt_id.to_string(),
                "connector": "biopay"
            }),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BiopayAuthenticateResponse {
    pub ok: bool,
    pub session_id: String,
    pub merchant_id: Option<String>,
    pub status: String,
    pub redirect_url: String,
    pub expires_at: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, BiopayAuthenticateResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, BiopayAuthenticateResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if !item.response.ok {
            return Err(errors::ConnectorError::ResponseHandlingFailed.into());
        }

        let mut form_fields = HashMap::new();
        form_fields.insert("session_id".to_string(), item.response.session_id.clone());

        Ok(Self {
            status: AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.session_id.clone()),
                redirection_data: Box::new(Some(RedirectForm::Form {
                    endpoint: item.response.redirect_url,
                    method: Method::Get,
                    form_fields,
                })),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: Some(item.response.session_id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct BiopayPostAuthenticateRequest {
    pub session_id: String,
}

impl TryFrom<&RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>>
    for BiopayPostAuthenticateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RouterData<api::PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let session_id = item
            .request
            .connector_transaction_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(Self { session_id })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BiopayPostAuthenticateResponse {
    pub ok: bool,
    pub session_id: String,
    pub status: String,
    pub platform: Option<String>,
    pub platform_merchant_id: Option<String>,
    pub platform_profile_id: Option<String>,
    pub platform_payment_id: Option<String>,
    pub biopay_customer_id: Option<String>,
    pub provider: Option<String>,
    pub token_reference: Option<String>,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub return_url: Option<String>,
    pub expires_at: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, BiopayPostAuthenticateResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, BiopayPostAuthenticateResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        if !item.response.ok {
            return Err(errors::ConnectorError::ResponseHandlingFailed.into());
        }

      let attempt_status = match item.response.status.as_str() {
    "approved" => AttemptStatus::AuthenticationSuccessful,
    "expired" => AttemptStatus::Expired,
    "failed" => AttemptStatus::AuthenticationFailed,
    "pending" => AttemptStatus::AuthenticationPending,
    unknown => {
        return Err(errors::ConnectorError::ResponseDeserializationFailed.into())
            .attach_printable(format!("Unexpected BioPay status: {}", unknown));
    }
};

        Ok(Self {
            status: attempt_status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.session_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: Some(item.response.session_id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BiopayErrorResponse {
    pub status_code: Option<u16>,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}
