use std::collections::HashMap;

pub use common_utils::request::Method;
use common_utils::{errors::CustomResult, ext_traits::ValueExt, pii::Email};
use error_stack::ResultExt;
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

fn get_mid(
    connector_auth_type: &types::ConnectorAuthType,
    payment_method_type: Option<enums::PaymentMethodType>,
    currency: enums::Currency,
) -> Result<Secret<String>, errors::ConnectorError> {
    match CashtocodeAuth::try_from((connector_auth_type, &currency)) {
        Ok(cashtocode_auth) => match payment_method_type {
            Some(enums::PaymentMethodType::ClassicReward) => Ok(cashtocode_auth
                .merchant_id_classic
                .ok_or(errors::ConnectorError::FailedToObtainAuthType)?),
            Some(enums::PaymentMethodType::Evoucher) => Ok(cashtocode_auth
                .merchant_id_evoucher
                .ok_or(errors::ConnectorError::FailedToObtainAuthType)?),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType),
        },
        Err(_) => Err(errors::ConnectorError::FailedToObtainAuthType)?,
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for CashtocodePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let customer_id = item.get_customer_id()?;
        let url = item.request.get_router_return_url()?;
        let mid = get_mid(
            &item.connector_auth_type,
            item.request.payment_method_type,
            item.request.currency,
        )?;
        match item.payment_method {
            diesel_models::enums::PaymentMethod::Reward => Ok(Self {
                amount: utils::to_currency_base_unit_asf64(
                    item.request.amount,
                    item.request.currency,
                )?,
                transaction_id: item.attempt_id.clone(),
                currency: item.request.currency,
                user_id: Secret::new(customer_id.to_owned()),
                first_name: None,
                last_name: None,
                user_alias: Secret::new(customer_id),
                requested_url: url.to_owned(),
                cancel_url: url,
                email: item.request.email.clone(),
                mid,
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

#[derive(Default, Debug, Deserialize)]
pub struct CashtocodeAuthType {
    pub auths: HashMap<enums::Currency, CashtocodeAuth>,
}

#[derive(Default, Debug, Deserialize)]
pub struct CashtocodeAuth {
    pub password_classic: Option<Secret<String>>,
    pub password_evoucher: Option<Secret<String>>,
    pub username_classic: Option<Secret<String>>,
    pub username_evoucher: Option<Secret<String>>,
    pub merchant_id_classic: Option<Secret<String>>,
    pub merchant_id_evoucher: Option<Secret<String>>,
}

impl TryFrom<&types::ConnectorAuthType> for CashtocodeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>; // Assuming ErrorStack is the appropriate error type

    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::CurrencyAuthKey { auth_key_map } => {
                let transformed_auths = auth_key_map
                    .iter()
                    .map(|(currency, identity_auth_key)| {
                        let cashtocode_auth = identity_auth_key
                            .to_owned()
                            .parse_value::<CashtocodeAuth>("CashtocodeAuth")
                            .change_context(errors::ConnectorError::InvalidDataFormat {
                                field_name: "auth_key_map",
                            })?;

                        Ok((currency.to_owned(), cashtocode_auth))
                    })
                    .collect::<Result<_, Self::Error>>()?;

                Ok(Self {
                    auths: transformed_auths,
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<(&types::ConnectorAuthType, &enums::Currency)> for CashtocodeAuth {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(value: (&types::ConnectorAuthType, &enums::Currency)) -> Result<Self, Self::Error> {
        let (auth_type, currency) = value;

        if let types::ConnectorAuthType::CurrencyAuthKey { auth_key_map } = auth_type {
            if let Some(identity_auth_key) = auth_key_map.get(currency) {
                let cashtocode_auth: Self = identity_auth_key
                    .to_owned()
                    .parse_value("CashtocodeAuth")
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(cashtocode_auth)
            } else {
                Err(errors::ConnectorError::CurrencyNotSupported {
                    message: currency.to_string(),
                    connector: "CashToCode",
                }
                .into())
            }
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
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

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CashtocodeErrors {
    pub message: String,
    pub path: String,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CashtocodePaymentsResponse {
    CashtoCodeError(CashtocodeErrorResponse),
    CashtoCodeData(CashtocodePaymentsResponseData),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodePaymentsResponseData {
    pub pay_url: url::Url,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodePaymentsSyncResponse {
    pub transaction_id: String,
    pub amount: f64,
}

fn get_redirect_form_data(
    payment_method_type: &enums::PaymentMethodType,
    response_data: CashtocodePaymentsResponseData,
) -> CustomResult<services::RedirectForm, errors::ConnectorError> {
    match payment_method_type {
        enums::PaymentMethodType::ClassicReward => Ok(services::RedirectForm::Form {
            //redirect form is manually constructed because the connector for this pm type expects query params in the url
            endpoint: response_data.pay_url.to_string(),
            method: services::Method::Post,
            form_fields: Default::default(),
        }),
        enums::PaymentMethodType::Evoucher => Ok(services::RedirectForm::from((
            //here the pay url gets parsed, and query params are sent as formfields as the connector expects
            response_data.pay_url,
            services::Method::Get,
        ))),
        _ => Err(errors::ConnectorError::NotImplemented(
            utils::get_unimplemented_payment_method_error_message("CashToCode"),
        ))?,
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            CashtocodePaymentsResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            CashtocodePaymentsResponse,
            types::PaymentsAuthorizeData,
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
                    attempt_status: None,
                    connector_transaction_id: None,
                }),
            ),
            CashtocodePaymentsResponse::CashtoCodeData(response_data) => {
                let payment_method_type = item
                    .data
                    .request
                    .payment_method_type
                    .as_ref()
                    .ok_or(errors::ConnectorError::MissingPaymentMethodType)?;
                let redirection_data = get_redirect_form_data(payment_method_type, response_data)?;
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
                        incremental_authorization_allowed: None,
                        integrity_object: None,
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
            status: enums::AttemptStatus::Charged, // Charged status is hardcoded because cashtocode do not support Psync, and we only receive webhooks when payment is succeeded, this tryFrom is used for CallConnectorAction.
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.data.attempt_id.clone(), //in response they only send PayUrl, so we use attempt_id as connector_transaction_id
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                integrity_object: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CashtocodeErrorResponse {
    pub error: serde_json::Value,
    pub error_description: String,
    pub errors: Option<Vec<CashtocodeErrors>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashtocodeIncomingWebhook {
    pub amount: f64,
    pub currency: String,
    pub foreign_transaction_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub transaction_id: String,
}
