use std::collections::HashMap;

use common_enums::{enums, Currency};
use common_utils::{ext_traits::OptionExt, pii, request::Method, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsSyncRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{PaymentsSyncResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData},
};

pub struct CoingateRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for CoingateRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoingateConnectorMetadataObject {
    pub currency_id: i32,
    pub platform_id: i32,
    pub ledger_account_id: Secret<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for CoingateConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct CoingatePaymentsRequest {
    price_amount: StringMajorUnit,
    price_currency: Currency,
    receive_currency: String,
    callback_url: String,
    success_url: Option<String>,
    cancel_url: Option<String>,
    title: String,
    token: Secret<String>,
}

impl TryFrom<&CoingateRouterData<&PaymentsAuthorizeRouterData>> for CoingatePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CoingateRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = CoingateAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(match item.router_data.request.payment_method_data {
            PaymentMethodData::Crypto(_) => Ok(Self {
                price_amount: item.amount.clone(),
                price_currency: item.router_data.request.currency,
                receive_currency: "DO_NOT_CONVERT".to_string(),
                callback_url: item.router_data.request.get_webhook_url()?,
                success_url: item.router_data.request.router_return_url.clone(),
                cancel_url: item.router_data.request.router_return_url.clone(),
                title: item.router_data.connector_request_reference_id.clone(),
                token: auth.merchant_token,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Coingate"),
            )),
        }?)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CoingateSyncResponse {
    status: CoingatePaymentStatus,
    id: i64,
}
impl TryFrom<PaymentsSyncResponseRouterData<CoingateSyncResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<CoingateSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.to_string()),
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
    }
}
pub struct CoingateAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_token: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CoingateAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                merchant_token: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CoingatePaymentStatus {
    New,
    Pending,
    Confirming,
    Paid,
    Invalid,
    Expired,
    Canceled,
}

impl From<CoingatePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: CoingatePaymentStatus) -> Self {
        match item {
            CoingatePaymentStatus::Paid => Self::Charged,
            CoingatePaymentStatus::Canceled
            | CoingatePaymentStatus::Expired
            | CoingatePaymentStatus::Invalid => Self::Failure,
            CoingatePaymentStatus::Confirming | CoingatePaymentStatus::New => {
                Self::AuthenticationPending
            }
            CoingatePaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoingatePaymentsResponse {
    status: CoingatePaymentStatus,
    id: i64,
    payment_url: Option<String>,
    order_id: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, CoingatePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CoingatePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.to_string()),
                redirection_data: Box::new(Some(RedirectForm::Form {
                    endpoint: item.response.payment_url.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "payment_url",
                        },
                    )?,
                    method: Method::Get,
                    form_fields: HashMap::new(),
                })),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct CoingateRefundRequest {
    pub amount: StringMajorUnit,
    pub address: Secret<String>,
    pub currency_id: i32,
    pub platform_id: i32,
    pub reason: String,
    pub email: pii::Email,
    pub ledger_account_id: Secret<String>,
}

impl<F> TryFrom<&CoingateRouterData<&RefundsRouterData<F>>> for CoingateRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CoingateRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let metadata: CoingateConnectorMetadataObject =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "merchant_connector_account.metadata",
                })?;

        let refund_metadata = item
            .router_data
            .request
            .refund_connector_metadata
            .as_ref()
            .get_required_value("refund_connector_metadata")
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "refund_connector_metadata",
            })?
            .clone()
            .expose();

        let address: Secret<String> = serde_json::from_value::<Secret<String>>(
            refund_metadata.get("address").cloned().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "address",
                }
            })?,
        )
        .change_context(errors::ConnectorError::MissingRequiredField {
            field_name: "address",
        })?;

        let email: pii::Email = serde_json::from_value::<pii::Email>(
            refund_metadata.get("email").cloned().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "email",
                }
            })?,
        )
        .change_context(errors::ConnectorError::MissingRequiredField {
            field_name: "email",
        })?;

        Ok(Self {
            amount: item.amount.clone(),
            address,
            currency_id: metadata.currency_id,
            platform_id: metadata.platform_id,
            reason: item.router_data.request.reason.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "refund.reason",
                },
            )?,
            email,
            ledger_account_id: metadata.ledger_account_id,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoingateRefundResponse {
    pub status: CoingateRefundStatus,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CoingateRefundStatus {
    Pending,
    Completed,
    Rejected,
    Processing,
}

impl TryFrom<RefundsResponseRouterData<Execute, CoingateRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, CoingateRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, CoingateRefundResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, CoingateRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl From<CoingateRefundStatus> for common_enums::RefundStatus {
    fn from(item: CoingateRefundStatus) -> Self {
        match item {
            CoingateRefundStatus::Pending => Self::Pending,
            CoingateRefundStatus::Completed => Self::Success,
            CoingateRefundStatus::Rejected => Self::Failure,
            CoingateRefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CoingateErrorResponse {
    pub message: String,
    pub reason: String,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoingateWebhookBody {
    pub token: Secret<String>,
    pub status: CoingatePaymentStatus,
    pub id: i64,
}
