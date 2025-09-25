use common_enums::{enums, Currency};
use common_utils::{
    id_type,
    pii::{self, Email, IpAddress},
    request::Method,
    types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, BrowserInformationData, PaymentsAuthorizeRequestData, RouterData as _},
};

pub struct GigadatRouterData<T> {
    pub amount: FloatMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for GigadatRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

const CONNECTOR_BASE_URL: &str = "https://interac.express-connect.com/";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GigadatConnectorMetadataObject {
    pub site: String,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for GigadatConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;
        Ok(metadata)
    }
}

// CPI (Combined Pay-in) Request Structure for Gigadat
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GigadatCpiRequest {
    pub user_id: id_type::CustomerId,
    pub site: String,
    pub user_ip: Secret<String, IpAddress>,
    pub currency: Currency,
    pub amount: FloatMajorUnit,
    pub transaction_id: String,
    #[serde(rename = "type")]
    pub transaction_type: GidadatTransactionType,
    pub name: Secret<String>,
    pub email: Email,
    pub mobile: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum GidadatTransactionType {
    Cpi,
}

impl TryFrom<&GigadatRouterData<&PaymentsAuthorizeRouterData>> for GigadatCpiRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GigadatRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata: GigadatConnectorMetadataObject =
            utils::to_connector_meta_from_secret(item.router_data.connector_meta_data.clone())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "merchant_connector_account.metadata",
                })?;
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankRedirect(BankRedirectData::Interac { .. }) => {
                let router_data = item.router_data;
                let name = router_data.get_billing_full_name()?;
                let email = router_data.get_billing_email()?;
                let mobile = router_data.get_billing_phone_number()?;
                let currency = item.router_data.request.currency;

                let user_ip = router_data.request.get_browser_info()?.get_ip_address()?;

                Ok(Self {
                    user_id: router_data.get_customer_id()?,
                    site: metadata.site,
                    user_ip,
                    currency,
                    amount: item.amount,
                    transaction_id: router_data.connector_request_reference_id.clone(),
                    transaction_type: GidadatTransactionType::Cpi,
                    name,
                    email,
                    mobile,
                })
            }
            PaymentMethodData::BankRedirect(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Gigadat"),
            ))?,

            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Gigadat"),
            )
            .into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GigadatAuthType {
    pub campaign_id: Secret<String>,
    pub username: Secret<String>,
    pub password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for GigadatAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                password: api_secret.to_owned(),
                username: api_key.to_owned(),
                campaign_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GigadatPaymentResponse {
    pub token: Secret<String>,
    pub data: GigadatPaymentData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GigadatPaymentData {
    pub transaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GigadatPaymentStatus {
    StatusInited,
    StatusSuccess,
    StatusRejected,
    StatusRejected1,
    StatusExpired,
    StatusAborted1,
    StatusPending,
    StatusFailed,
}

impl From<GigadatPaymentStatus> for enums::AttemptStatus {
    fn from(item: GigadatPaymentStatus) -> Self {
        match item {
            GigadatPaymentStatus::StatusSuccess => Self::Charged,
            GigadatPaymentStatus::StatusInited | GigadatPaymentStatus::StatusPending => {
                Self::Pending
            }
            GigadatPaymentStatus::StatusRejected
            | GigadatPaymentStatus::StatusExpired
            | GigadatPaymentStatus::StatusRejected1
            | GigadatPaymentStatus::StatusAborted1
            | GigadatPaymentStatus::StatusFailed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GigadatTransactionStatusResponse {
    pub status: GigadatPaymentStatus,
}

impl<F, T> TryFrom<ResponseRouterData<F, GigadatPaymentResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, GigadatPaymentResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Will be raising a sepearte PR to populate a field connect_base_url in routerData and use it here
        let base_url = CONNECTOR_BASE_URL;

        let redirect_url = format!(
            "{}/webflow?transaction={}&token={}",
            base_url,
            item.data.connector_request_reference_id,
            item.response.token.peek()
        );

        let redirection_data = Some(RedirectForm::Form {
            endpoint: redirect_url,
            method: Method::Get,
            form_fields: Default::default(),
        });
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.data.transaction_id),
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

impl<F, T> TryFrom<ResponseRouterData<F, GigadatTransactionStatusResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, GigadatTransactionStatusResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
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

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct GigadatRefundRequest {
    pub amount: FloatMajorUnit,
    pub transaction_id: String,
    pub campaign_id: Secret<String>,
}

impl<F> TryFrom<&GigadatRouterData<&RefundsRouterData<F>>> for GigadatRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &GigadatRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth_type = GigadatAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            amount: item.amount.to_owned(),
            transaction_id: item.router_data.request.connector_transaction_id.clone(),
            campaign_id: auth_type.campaign_id,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    success: bool,
    data: GigadatPaymentData,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.http_code {
            200 => enums::RefundStatus::Success,
            400 | 401 | 422 => enums::RefundStatus::Failure,
            _ => enums::RefundStatus::Pending,
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.data.transaction_id.to_string(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct GigadatErrorResponse {
    pub err: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct GigadatRefundErrorResponse {
    pub error: Vec<Error>,
    pub message: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Error {
    pub code: Option<String>,
    pub detail: String,
}
