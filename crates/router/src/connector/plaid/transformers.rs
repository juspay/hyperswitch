use common_enums::Currency;
use common_utils::types::FloatMajorUnit;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::is_payment_failure,
    core::errors,
    types::{self, api, domain, storage::enums},
};

pub struct PlaidRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for PlaidRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct PlaidPaymentsRequest {
    amount: PlaidAmount,
    recipient_id: String,
    reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule: Option<PlaidSchedule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<PlaidOptions>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidAmount {
    currency: Currency,
    value: FloatMajorUnit,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidSchedule {
    interval: String,
    interval_execution_day: String,
    start_date: String,
    end_date: Option<String>,
    adjusted_start_date: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidOptions {
    request_refund_details: bool,
    iban: Option<Secret<String>>,
    bacs: Option<PlaidBacs>,
    scheme: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidBacs {
    account: Secret<String>,
    sort_code: Secret<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlaidLinkTokenRequest {
    client_name: String,
    country_codes: Vec<String>,
    language: String,
    products: Vec<String>,
    user: User,
    payment_initiation: PlaidPaymentInitiation,
    redirect_uri: Option<String>,
    android_package_name: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct User {
    pub client_user_id: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidPaymentInitiation {
    payment_id: String,
}

impl TryFrom<&PlaidRouterData<&types::PaymentsAuthorizeRouterData>> for PlaidPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PlaidRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::OpenBanking(ref data) => match data {
                domain::OpenBankingData::OpenBankingPIS { .. } => {
                    let amount = item.amount;
                    let currency = item.router_data.request.currency;
                    let payment_id = item.router_data.payment_id.clone();
                    let id_len = payment_id.len();
                    let reference = if id_len > 18 {
                        payment_id.get(id_len - 18..id_len).map(|id| id.to_string())
                    } else {
                        Some(payment_id)
                    }
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "payment_id",
                    })?;
                    let recipient_type = item
                        .router_data
                        .additional_merchant_data
                        .as_ref()
                        .map(|merchant_data| match merchant_data {
                            api_models::admin::AdditionalMerchantData::OpenBankingRecipientData(
                                data,
                            ) => data.clone(),
                        })
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "additional_merchant_data",
                        })?;

                    let recipient_id = match recipient_type {
                        api_models::admin::MerchantRecipientData::ConnectorRecipientId(id) => {
                            Ok(id.peek().to_string())
                        }
                        _ => Err(errors::ConnectorError::MissingRequiredField {
                            field_name: "ConnectorRecipientId",
                        }),
                    }?;

                    Ok(Self {
                        amount: PlaidAmount {
                            currency,
                            value: amount,
                        },
                        reference,
                        recipient_id,
                        schedule: None,
                        options: None,
                    })
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for PlaidSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        match item.request.connector_transaction_id {
            types::ResponseId::ConnectorTransactionId(ref id) => Ok(Self {
                payment_id: id.clone(),
            }),
            _ => Err((errors::ConnectorError::MissingConnectorTransactionID).into()),
        }
    }
}

impl TryFrom<&types::PaymentsPostProcessingRouterData> for PlaidLinkTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsPostProcessingRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            domain::PaymentMethodData::OpenBanking(ref data) => match data {
                domain::OpenBankingData::OpenBankingPIS { .. } => {
                    let headers = item.header_payload.clone();

                    let platform = headers
                        .as_ref()
                        .and_then(|headers| headers.x_client_platform.clone());

                    let (is_android, is_ios) = match platform {
                        Some(common_enums::ClientPlatform::Android) => (true, false),
                        Some(common_enums::ClientPlatform::Ios) => (false, true),
                        _ => (false, false),
                    };

                    Ok(Self {
                        client_name: "Hyperswitch".to_string(),
                        country_codes: item
                            .request
                            .country
                            .map(|code| vec![code.to_string()])
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "billing.address.country",
                            })?,
                        language: "en".to_string(),
                        products: vec!["payment_initiation".to_string()],
                        user: User {
                            client_user_id: item
                                .request
                                .customer_id
                                .clone()
                                .map(|id| id.get_string_repr().to_string())
                                .unwrap_or("default cust".to_string()),
                        },
                        payment_initiation: PlaidPaymentInitiation {
                            payment_id: item
                                .request
                                .connector_transaction_id
                                .clone()
                                .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
                        },
                        android_package_name: if is_android {
                            headers
                                .as_ref()
                                .and_then(|headers| headers.x_app_id.clone())
                        } else {
                            None
                        },
                        redirect_uri: if is_ios {
                            headers
                                .as_ref()
                                .and_then(|headers| headers.x_redirect_uri.clone())
                        } else {
                            None
                        },
                    })
                }
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct PlaidAuthType {
    pub client_id: Secret<String>,
    pub secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for PlaidAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                client_id: api_key.to_owned(),
                secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(strum::Display)]
pub enum PlaidPaymentStatus {
    PaymentStatusInputNeeded,
    PaymentStatusInitiated,
    PaymentStatusInsufficientFunds,
    PaymentStatusFailed,
    PaymentStatusBlocked,
    PaymentStatusCancelled,
    PaymentStatusExecuted,
    PaymentStatusSettled,
    PaymentStatusEstablished,
    PaymentStatusRejected,
    PaymentStatusAuthorising,
}

impl From<PlaidPaymentStatus> for enums::AttemptStatus {
    fn from(item: PlaidPaymentStatus) -> Self {
        match item {
            PlaidPaymentStatus::PaymentStatusAuthorising => Self::Authorizing,
            PlaidPaymentStatus::PaymentStatusBlocked
            | PlaidPaymentStatus::PaymentStatusInsufficientFunds
            | PlaidPaymentStatus::PaymentStatusRejected => Self::AuthorizationFailed,
            PlaidPaymentStatus::PaymentStatusCancelled => Self::Voided,
            PlaidPaymentStatus::PaymentStatusEstablished => Self::Authorized,
            PlaidPaymentStatus::PaymentStatusExecuted
            | PlaidPaymentStatus::PaymentStatusSettled
            | PlaidPaymentStatus::PaymentStatusInitiated => Self::Charged,
            PlaidPaymentStatus::PaymentStatusFailed => Self::Failure,
            PlaidPaymentStatus::PaymentStatusInputNeeded => Self::AuthenticationPending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaidPaymentsResponse {
    status: PlaidPaymentStatus,
    payment_id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PlaidPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status.clone());
        Ok(Self {
            status,
            response: if is_payment_failure(status) {
                Err(types::ErrorResponse {
                    // populating status everywhere as plaid only sends back a status
                    code: item.response.status.clone().to_string(),
                    message: item.response.status.clone().to_string(),
                    reason: Some(item.response.status.to_string()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.payment_id),
                })
            } else {
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.payment_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.payment_id),
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            },
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidLinkTokenResponse {
    link_token: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidLinkTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let session_token = Some(api::OpenBankingSessionToken {
            open_banking_session_token: item.response.link_token,
        });

        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(types::PaymentsResponseData::PostProcessingResponse { session_token }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PlaidSyncRequest {
    payment_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaidSyncResponse {
    payment_id: String,
    amount: PlaidAmount,
    status: PlaidPaymentStatus,
    recipient_id: String,
    reference: String,
    last_status_update: String,
    adjusted_reference: Option<String>,
    schedule: Option<PlaidSchedule>,
    iban: Option<Secret<String>>,
    bacs: Option<PlaidBacs>,
    scheme: Option<String>,
    adjusted_scheme: Option<String>,
    request_id: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PlaidSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PlaidSyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status.clone());
        Ok(Self {
            status,
            response: if is_payment_failure(status) {
                Err(types::ErrorResponse {
                    // populating status everywhere as plaid only sends back a status
                    code: item.response.status.clone().to_string(),
                    message: item.response.status.clone().to_string(),
                    reason: Some(item.response.status.to_string()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.payment_id),
                })
            } else {
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.payment_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.payment_id),
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            },
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PlaidErrorResponse {
    pub display_message: Option<String>,
    pub error_code: Option<String>,
    pub error_message: String,
    pub error_type: Option<String>,
}
