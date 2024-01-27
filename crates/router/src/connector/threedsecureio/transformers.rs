use error_stack::{report, IntoReport, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{CardData, PaymentsAuthorizeRequestData},
    core::errors,
    types::{self, api, storage::enums, transformers::ForeignTryFrom},
};

//TODO: Fill the struct with respective fields
pub struct ThreedsecureioRouterData<T> {
    pub amount: i64, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for ThreedsecureioRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

impl<T> TryFrom<(i64, T)> for ThreedsecureioRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(router_data: (i64, T)) -> Result<Self, Self::Error> {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Ok(Self {
            amount: router_data.0,
            router_data: router_data.1,
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ThreedsecureioPaymentsRequest {
    amount: i64,
    card: ThreedsecureioCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ThreedsecureioCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&ThreedsecureioRouterData<&types::PaymentsAuthorizeRouterData>>
    for ThreedsecureioPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ThreedsecureioRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = ThreedsecureioCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.to_owned(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ThreedsecureioAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for ThreedsecureioAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThreedsecureioPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<ThreedsecureioPaymentStatus> for enums::AttemptStatus {
    fn from(item: ThreedsecureioPaymentStatus) -> Self {
        match item {
            ThreedsecureioPaymentStatus::Succeeded => Self::Charged,
            ThreedsecureioPaymentStatus::Failed => Self::Failure,
            ThreedsecureioPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreedsecureioPaymentsResponse {
    status: ThreedsecureioPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            ThreedsecureioPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            ThreedsecureioPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ThreedsecureioRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&ThreedsecureioRouterData<&types::RefundsRouterData<F>>>
    for ThreedsecureioRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ThreedsecureioRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
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

fn get_card_details(
    payment_method_data: api_models::payments::PaymentMethodData,
) -> Result<api_models::payments::Card, errors::ConnectorError> {
    match payment_method_data {
        api_models::payments::PaymentMethodData::Card(details) => Ok(details),
        _ => Err(errors::ConnectorError::RequestEncodingFailed)?,
    }
}

impl TryFrom<&ThreedsecureioRouterData<&types::ConnectorAuthenticationRouterData>>
    for ThreedsecureioAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ThreedsecureioRouterData<&types::ConnectorAuthenticationRouterData>,
    ) -> Result<Self, Self::Error> {
        let card_details = get_card_details(item.router_data.request.payment_method_data.clone())?;
        Ok(Self {
            ds_start_protocol_version: "2.1.0".to_string(),
            ds_end_protocol_version: "2.1.0".to_string(),
            acs_start_protocol_version: "2.1.0".to_string(),
            acs_end_protocol_version: "2.1.0".to_string(),
            three_dsserver_trans_id: item.router_data.request.three_ds_server_trans_id.clone(),
            // acct_number: card_details.card_number.to_string(),
            acct_number: "3000100811111072".to_string(),
            notification_url: "https://webhook.site/8d03e3ea-a7d8-48f5-a200-476bca75a55c"
                .to_string(),
            three_dscomp_ind: "Y".to_string(),
            // three_dsrequestor_url: todo!(),
            acquirer_bin: item
                .router_data
                .request
                .acquirer_details
                .clone()
                .map(|acquirer| acquirer.acquirer_bin)
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
            acquirer_merchant_id: item
                .router_data
                .request
                .acquirer_details
                .clone()
                .map(|acquirer| acquirer.acquirer_merchant_mid)
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
            card_expiry_date: card_details.get_expiry_date_as_yymm()?.expose(),
            bill_addr_city: item
                .router_data
                .request
                .billing_address
                .city
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                .to_string(),
            bill_addr_country: item
                .router_data
                .request
                .billing_address
                .country
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                .to_string(),
            bill_addr_line1: "Bill Address Line 1".to_string(),
            bill_addr_post_code: "Bill Post Code".to_string(),
            bill_addr_state: "CO".to_string(),
            three_dsrequestor_authentication_ind: "01".to_string(),
            device_channel: "02".to_string(),
            browser_javascript_enabled: true,
            browser_accept_header:
                "text/html,application/xhtml+xml,application/xml; q=0.9,*/*;q=0.8".to_string(),
            browser_ip: "192.168.1.11".to_string(),
            browser_java_enabled: true,
            browser_language: "en".to_string(),
            browser_color_depth: "48".to_string(),
            browser_screen_height: "400".to_string(),
            browser_screen_width: "600".to_string(),
            browser_tz: "0".to_string(),
            browser_user_agent:
                "Mozilla/5.0 (Windows NT 6.1; Win64; x64; rv:47.0) Gecko/20100101 Firefox/47.0"
                    .to_string(),
            mcc: "5411".to_string(),
            merchant_country_code: "840".to_string(),
            merchant_name: "Dummy Merchant".to_string(),
            message_category: "01".to_string(),
            message_type: "AReq".to_string(),
            message_version: "2.1.0".to_string(),
            purchase_amount: item.amount.to_string(),
            purchase_currency: "840".to_string(),
            trans_type: "01".to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioErrorResponse {
    pub error_code: String,
    pub error_component: String,
    pub error_description: String,
    pub error_detail: String,
    pub error_message_type: String,
    pub message_type: String,
    pub message_version: String,
    pub three_dsserver_trans_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioAuthenticationResponse {
    pub acs_challenge_mandated: String,
    pub acs_operator_id: String,
    pub acs_reference_number: String,
    pub acs_trans_id: String,
    pub acs_url: url::Url,
    pub authentication_type: String,
    pub ds_reference_number: String,
    pub ds_trans_id: String,
    pub message_type: String,
    pub message_version: String,
    pub three_dsserver_trans_id: String,
    pub trans_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioAuthenticationRequest {
    pub ds_start_protocol_version: String,
    pub ds_end_protocol_version: String,
    pub acs_start_protocol_version: String,
    pub acs_end_protocol_version: String,
    pub three_dsserver_trans_id: String,
    pub acct_number: String,
    pub notification_url: String,
    pub three_dscomp_ind: String,
    // pub three_dsrequestor_url: String,
    pub acquirer_bin: String,
    pub acquirer_merchant_id: String,
    pub card_expiry_date: String,
    pub bill_addr_city: String,
    pub bill_addr_country: String,
    pub bill_addr_line1: String,
    pub bill_addr_post_code: String,
    pub bill_addr_state: String,
    // pub email: Email,
    pub three_dsrequestor_authentication_ind: String,
    // pub cardholder_name: Secret<String>,
    pub device_channel: String,
    pub browser_javascript_enabled: bool,
    pub browser_accept_header: String,
    pub browser_ip: String,
    pub browser_java_enabled: bool,
    pub browser_language: String,
    pub browser_color_depth: String,
    pub browser_screen_height: String,
    pub browser_screen_width: String,
    pub browser_tz: String,
    pub browser_user_agent: String,
    pub mcc: String,
    pub merchant_country_code: String,
    pub merchant_name: String,
    pub message_category: String,
    pub message_type: String,
    pub message_version: String,
    pub purchase_amount: String,
    pub purchase_currency: String,
    // pub purchase_exponent: String,
    // pub purchase_date: String,
    pub trans_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioPreAuthenticationRequest {
    acct_number: String,
    ds: Option<DirectoryServer>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DirectoryServer {
    Standin,
    Visa,
    Mastercard,
    Jcb,
    Upi,
    Amex,
    Protectbuy,
    Sbn,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreedsecureioPreAuthenticationResponse {
    pub ds_start_protocol_version: String,
    pub ds_end_protocol_version: String,
    pub acs_start_protocol_version: String,
    pub acs_end_protocol_version: String,
    #[serde(rename = "threeDSMethodURL")]
    pub threeds_method_url: Option<String>,
    #[serde(rename = "threeDSServerTransID")]
    pub threeds_server_trans_id: String,
    pub scheme: String,
    pub message_type: String,
}

impl TryFrom<&ThreedsecureioRouterData<&types::authentication::PreAuthNRouterData>>
    for ThreedsecureioPreAuthenticationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &ThreedsecureioRouterData<&types::authentication::PreAuthNRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = value.router_data;
        Ok(Self {
            acct_number: router_data
                .request
                .card_holder_account_number
                .clone()
                .get_card_no(),
            ds: None,
        })
    }
}

impl ForeignTryFrom<String> for (i64, i64, i64) {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(value: String) -> Result<Self, Self::Error> {
        let mut splitted_version = value.split('.');
        let version_string = {
            let major_version = splitted_version.next().ok_or(report!(
                errors::ConnectorError::ResponseDeserializationFailed
            ))?;
            let minor_version = splitted_version.next().ok_or(report!(
                errors::ConnectorError::ResponseDeserializationFailed
            ))?;
            let patch_version = splitted_version.next().ok_or(report!(
                errors::ConnectorError::ResponseDeserializationFailed
            ))?;
            (major_version, minor_version, patch_version)
        };
        let int_representation = {
            let major_version = version_string
                .0
                .parse()
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            let minor_version = version_string
                .1
                .parse()
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            let patch_version = version_string
                .2
                .parse()
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
            (major_version, minor_version, patch_version)
        };
        Ok(int_representation)
    }
}
