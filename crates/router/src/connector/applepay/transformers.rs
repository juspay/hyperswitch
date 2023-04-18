use api_models::payments;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use masking::{Deserialize, Serialize};

use crate::{connector::utils, core::errors, types, utils::OptionExt};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepaySessionRequest {
    merchant_identifier: String,
    display_name: String,
    initiative: String,
    initiative_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepaySessionTokenResponse {
    pub epoch_timestamp: u64,
    pub expires_at: u64,
    pub merchant_session_identifier: String,
    pub nonce: String,
    pub merchant_identifier: String,
    pub domain_name: String,
    pub display_name: String,
    pub signature: String,
    pub operational_analytics_identifier: String,
    pub retries: u8,
    pub psp_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub status_code: String,
    pub status_message: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApplePayMetadata {
    pub payment_request_data: PaymentRequestMetadata,
    pub session_token_data: SessionRequest,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PaymentRequestMetadata {
    pub supported_networks: Vec<String>,
    pub merchant_capabilities: Vec<String>,
    pub label: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionRequest {
    pub certificate: String,
    pub certificate_keys: String,
    pub merchant_identifier: String,
    pub display_name: String,
    pub initiative: String,
    pub initiative_context: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PaymentRequest {
    pub apple_pay_merchant_id: String,
    pub country_code: api_models::enums::CountryCode,
    pub currency_code: String,
    pub total: AmountInfo,
    pub merchant_capabilities: Vec<String>,
    pub supported_networks: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AmountInfo {
    pub label: String,
    #[serde(rename = "type")]
    pub label_type: String,
    pub amount: String,
}

impl TryFrom<&types::PaymentsSessionRouterData> for ApplepaySessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSessionRouterData) -> Result<Self, Self::Error> {
        let metadata = item
            .connector_meta_data
            .to_owned()
            .get_required_value("connector_meta_data")
            .change_context(errors::ConnectorError::NoConnectorMetaData)?;

        let metadata: ApplePayMetadata = metadata
            .parse_value("ApplePayMetadata")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Self {
            merchant_identifier: metadata.session_token_data.merchant_identifier,
            display_name: metadata.session_token_data.display_name,
            initiative: metadata.session_token_data.initiative,
            initiative_context: metadata.session_token_data.initiative_context,
        })
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            ApplepaySessionTokenResponse,
            types::PaymentsSessionData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsSessionData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            ApplepaySessionTokenResponse,
            types::PaymentsSessionData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let metadata = item
            .data
            .connector_meta_data
            .to_owned()
            .get_required_value("connector_meta_data")
            .change_context(errors::ConnectorError::NoConnectorMetaData)?;

        let metadata: ApplePayMetadata = metadata
            .parse_value("ApplePayMetadata")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let amount_info = AmountInfo {
            label: metadata.payment_request_data.label,
            label_type: "final".to_string(),
            amount: utils::to_currency_base_unit(
                item.data.request.amount,
                item.data.request.currency,
            )?,
        };

        let payment_request = PaymentRequest {
            country_code: item
                .data
                .request
                .country
                .to_owned()
                .get_required_value("country_code")
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "country_code",
                })?,
            currency_code: item.data.request.currency.to_string(),
            total: amount_info,
            merchant_capabilities: metadata.payment_request_data.merchant_capabilities,
            supported_networks: metadata.payment_request_data.supported_networks,
            apple_pay_merchant_id: metadata.session_token_data.merchant_identifier,
        };

        let applepay_session = ApplepaySessionTokenResponse {
            epoch_timestamp: item.response.epoch_timestamp,
            expires_at: item.response.expires_at,
            merchant_session_identifier: item.response.merchant_session_identifier,
            nonce: item.response.nonce,
            merchant_identifier: item.response.merchant_identifier,
            domain_name: item.response.domain_name,
            display_name: item.response.display_name,
            signature: item.response.signature,
            operational_analytics_identifier: item.response.operational_analytics_identifier,
            retries: item.response.retries,
            psp_id: item.response.psp_id,
        };

        Ok(Self {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: {
                    api_models::payments::SessionToken::ApplePay(Box::new(
                        payments::ApplepaySessionTokenResponse {
                            session_token_data: applepay_session.into(),
                            payment_request_data: payment_request.into(),
                        },
                    ))
                },
            }),
            ..item.data
        })
    }
}

impl From<PaymentRequest> for payments::ApplePayPaymentRequest {
    fn from(value: PaymentRequest) -> Self {
        Self {
            country_code: value.country_code,
            currency_code: value.currency_code,
            total: value.total.into(),
            merchant_capabilities: value.merchant_capabilities,
            supported_networks: value.supported_networks,
        }
    }
}

impl From<AmountInfo> for payments::AmountInfo {
    fn from(value: AmountInfo) -> Self {
        Self {
            label: value.label,
            total_type: value.label_type,
            amount: value.amount,
        }
    }
}

impl From<ApplepaySessionTokenResponse> for payments::ApplePaySessionResponse {
    fn from(value: ApplepaySessionTokenResponse) -> Self {
        Self {
            epoch_timestamp: value.epoch_timestamp,
            expires_at: value.expires_at,
            merchant_session_identifier: value.merchant_session_identifier,
            nonce: value.nonce,
            merchant_identifier: value.merchant_identifier,
            domain_name: value.domain_name,
            display_name: value.display_name,
            signature: value.signature,
            operational_analytics_identifier: value.operational_analytics_identifier,
            retries: value.retries,
            psp_id: value.psp_id,
        }
    }
}
