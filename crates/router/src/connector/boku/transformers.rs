use std::fmt;

use api_models::{
    enums::{CountryAlpha2, Currency},
    payments::Amount,
};
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

//use common_utils::ext_traits::XmlExt;
use crate::{
    core::errors,
    services,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BokuPaymentsRequest {
    BeginSingleCharge(SingleChargeData),
    // MultiCharge(MultiChargeData),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(rename = "begin-single-charge")]
pub struct SingleChargeData {
    total_amount: i64,
    currency: String,
    country: String,
    merchant_id: Secret<String>,
    merchant_request_id: Secret<String>,
    merchant_item_description: String,
    payment_method: String,
    charge_type: String,
    hosted: Option<BokuHostedData>,
}

#[derive(Debug, Clone, Serialize)]
pub enum BokuPaymentType {
    // PayPay,
    // LinePay,
    // RakutenPay,
    AuPay,
}

impl fmt::Display for BokuPaymentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuPay => write!(f, "aupay"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum BokuChargeType {
    Hosted,
    // #[serde(rename = "validate-optin")]
    // Validate,
    // #[serde(rename = "confirm-optin")]
    // Confirm,
}

impl fmt::Display for BokuChargeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hosted => write!(f, "hosted"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
struct BokuHostedData {
    forward_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MultiChargeData {
    total_amount: Amount,
    currency: Currency,
    country: CountryAlpha2,
    merchant_id: Secret<String>,
    merchant_request_id: String,
    merchant_item_description: String,
    optin_id: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BokuPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let country = match get_country_code(item) {
            Some(cn_code) => cn_code.to_string(),
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "country",
            })?,
        };
        let hosted = get_hosted_data(item);
        let auth_type = BokuAuthType::try_from(&item.connector_auth_type)?;
        let merchant_item_description = get_item_description(item)?;
        let payment_data = SingleChargeData {
            total_amount: item.request.amount,
            currency: item.request.currency.to_string(),
            country,
            merchant_id: auth_type.merchant_id,
            merchant_request_id: Secret::new(item.payment_id.to_string()),
            merchant_item_description,
            payment_method: BokuPaymentType::AuPay.to_string(),
            charge_type: BokuChargeType::Hosted.to_string(),
            hosted,
        };

        match item.request.payment_method_data {
            api_models::payments::PaymentMethodData::Wallet(ref wallet_data) => match wallet_data {
                api_models::payments::WalletData::MbWayRedirect { .. } => {
                    Ok(Self::BeginSingleCharge(payment_data))
                }
                _ => {
                    Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into())
                }
            },
            _ => Err(errors::ConnectorError::NotSupported {
                message: format!("{:?}", item.request.payment_method_type),
                connector: "Boku",
                payment_experience: api_models::enums::PaymentExperience::RedirectToUrl.to_string(),
            })?,
        }
    }
}

pub struct BokuAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) key_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for BokuAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_id: Secret::new(key1.to_string()),
                key_id: Secret::new(api_key.to_string()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Connector Meta Data
#[derive(Debug, Clone, Deserialize)]
pub struct BokuMetaData {
    pub(super) country: String,
}

#[derive(Debug, Clone, Deserialize)]
pub enum BokuPaymentStatus {
    #[serde(rename = "0")]
    Success,
    #[serde(rename = "3")]
    Failure,
}

impl From<BokuPaymentStatus> for enums::AttemptStatus {
    fn from(item: BokuPaymentStatus) -> Self {
        match item {
            BokuPaymentStatus::Success => Self::Charged,
            BokuPaymentStatus::Failure => Self::Failure,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "begin-single-charge-response")]
#[serde(rename_all = "kebab-case")]
pub struct BokuPaymentsResponse {
    result: ResultData,
    // merchant_id: String,
    // merchant_request_id: String,
    // payment_method: String,
    charge_id: String,
    hosted: Option<HostedUrlResponse>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResultData {
    #[serde(rename = "@value")]
    reason_code: BokuPaymentStatus,
    // message: String,
    // retriable: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HostedUrlResponse {
    redirect_url: Option<Url>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BokuPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, BokuPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = match item.response.hosted {
            Some(hosted_value) => Ok(hosted_value
                .redirect_url
                .map(|url| services::RedirectForm::from((url, services::Method::Get)))),
            None => Err(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "redirect_url",
            }),
        }?;

        Ok(Self {
            status: enums::AttemptStatus::from(item.response.result.reason_code),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.charge_id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct BokuRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BokuRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BokuErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BokuConnMetaData {
    country: String,
}

fn get_country_code(item: &types::PaymentsAuthorizeRouterData) -> Option<CountryAlpha2> {
    item.address
        .billing
        .as_ref()
        .and_then(|billing| billing.address.as_ref().and_then(|address| address.country))
}

fn get_hosted_data(item: &types::PaymentsAuthorizeRouterData) -> Option<BokuHostedData> {
    item.return_url
        .clone()
        .map(|url| BokuHostedData { forward_url: url })
}

fn get_item_description(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    match item.description.clone() {
        Some(desc) => Ok(desc),
        None => Err(errors::ConnectorError::MissingRequiredField {
            field_name: "description",
        })?,
    }
}
