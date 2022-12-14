use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use masking::{Deserialize, Serialize};

use crate::{core::errors, types, utils::OptionExt};

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
pub struct ApplepaySessionResponse {
    epoch_timestamp: u64,
    expires_at: u64,
    merchant_session_identifier: String,
    nonce: String,
    merchant_identifier: String,
    domain_name: String,
    display_name: String,
    signature: String,
    operational_analytics_identifier: String,
    retries: u8,
    psp_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub status_code: String,
    pub status_message: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionObject {
    pub certificate: String,
    pub certificate_keys: String,
    pub merchant_identifier: String,
    pub display_name: String,
    pub initiative: String,
    pub initiative_context: String,
}

impl TryFrom<&types::PaymentsSessionRouterData> for ApplepaySessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSessionRouterData) -> Result<Self, Self::Error> {
        let metadata = item
            .connector_meta_data
            .to_owned()
            .get_required_value("connector_meta_data")
            .change_context(errors::ConnectorError::NoConnectorMetaData)?;

        let session_object: SessionObject = metadata
            .parse_value("SessionObject")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        Ok(Self {
            merchant_identifier: session_object.merchant_identifier,
            display_name: session_object.display_name,
            initiative: session_object.initiative,
            initiative_context: session_object.initiative_context,
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, ApplepaySessionResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, ApplepaySessionResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(types::RouterData {
            //TODO : change in session response to fit apple pay session object
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: {
                    api_models::payments::SessionToken::Applepay {
                        epoch_timestamp: item.response.epoch_timestamp,
                        expires_at: item.response.expires_at,
                        merchant_session_identifier: item.response.merchant_session_identifier,
                        nonce: item.response.nonce,
                        merchant_identifier: item.response.merchant_identifier,
                        domain_name: item.response.domain_name,
                        display_name: item.response.display_name,
                        signature: item.response.signature,
                        operational_analytics_identifier: item
                            .response
                            .operational_analytics_identifier,
                        retries: item.response.retries,
                        psp_id: item.response.psp_id,
                    }
                },
            }),
            ..item.data
        })
    }
}
