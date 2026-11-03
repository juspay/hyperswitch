//! Standalone 3DS on `/v1/threeDSecure`: authenticate a card without paying, then read the
//! result back.

use common_enums::enums;
use common_utils::types::SemanticVersion;
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::authentication::{PostAuthentication, PreAuthentication},
    router_request_types::authentication::{
        AuthNFlowType, ConnectorPostAuthenticationRequestData, PreAuthNRequestData,
    },
    router_response_types::AuthenticationResponseData,
};
use hyperswitch_masking::Secret;
use serde::Serialize;

use super::{
    payment::card_details,
    request::{AciPaymentType, CardDetails},
    response::AciPaymentsResponse,
    result::ResultClass,
    three_ds::AciThreeDSecureResponse,
    AciAuthType, Error,
};
use crate::types::{PreAuthNRouterData, ResponseRouterData};

/// Request body for `POST /v1/threeDSecure` (standalone 3DS authentication).
#[derive(Debug, Serialize)]
pub struct AciStandaloneThreeDsRequest {
    #[serde(rename = "entityId")]
    pub entity_id: Secret<String>,
    /// Amount (optional for NPA flows).
    #[serde(rename = "amount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Currency (optional for NPA flows).
    #[serde(rename = "currency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "paymentType")]
    pub payment_type: AciPaymentType,
    #[serde(flatten)]
    pub card: CardDetails,
    /// Non-Payment Authentication flag.
    #[serde(rename = "threeDSecure.npa")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_ds_npa: Option<bool>,
    #[serde(rename = "shopperResultUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopper_result_url: Option<String>,
}

impl TryFrom<&PreAuthNRouterData> for AciStandaloneThreeDsRequest {
    type Error = Error;

    fn try_from(item: &PreAuthNRouterData) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.connector_auth_type)?;
        let card_details = card_details(&item.request.card)?;
        Ok(Self {
            entity_id: auth.entity_id,
            amount: None,
            currency: None,
            payment_type: AciPaymentType::Preauthorization,
            card: card_details,
            three_ds_npa: Some(true),
            shopper_result_url: None,
        })
    }
}

/// Map ACI payment response to PreAuthentication RouterData response.
impl
    TryFrom<
        ResponseRouterData<
            PreAuthentication,
            AciPaymentsResponse,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
    > for RouterData<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
{
    type Error = Error;

    fn try_from(
        item: ResponseRouterData<
            PreAuthentication,
            AciPaymentsResponse,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = if item.response.result.is_failure() {
            Err(item.response.result.decline(
                &item.response.id,
                item.response.result_details.as_ref(),
                item.http_code,
            ))
        } else if let Some(tds) = item.response.three_d_secure.as_ref() {
            // Frictionless: 3DS completed without redirect
            Ok(AuthenticationResponseData::AuthNResponse {
                authn_flow_type: AuthNFlowType::Frictionless,
                authentication_value: tds.authentication_value(),
                trans_status: enums::TransactionStatus::Success,
                connector_metadata: None,
                ds_trans_id: tds.ds_transaction_id.clone(),
                eci: tds.eci.clone(),
                challenge_code: None,
                challenge_cancel: None,
                challenge_code_reason: None,
                message_extension: None,
            })
        } else {
            // Pending/redirect: challenge required
            let three_ds_method_url = item
                .response
                .redirect
                .as_ref()
                .and_then(|r| r.preconditions.as_ref())
                .and_then(|p| p.first())
                .map(|p| p.url.to_string());
            let version = item
                .response
                .three_d_secure
                .as_ref()
                .and_then(|tds| tds.version.as_ref())
                .and_then(|v| v.parse::<SemanticVersion>().ok())
                .unwrap_or(SemanticVersion::new(2, 0, 0));
            Ok(AuthenticationResponseData::PreAuthNResponse {
                threeds_server_transaction_id: item.response.id.clone(),
                maximum_supported_3ds_version: version.clone(),
                connector_authentication_id: item.response.id.clone(),
                three_ds_method_data: None,
                three_ds_method_url,
                message_version: version,
                connector_metadata: None,
                directory_server_id: None,
                scheme_id: None,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

/// Map ACI payment response to PostAuthentication RouterData response.
impl
    TryFrom<
        ResponseRouterData<
            PostAuthentication,
            AciPaymentsResponse,
            ConnectorPostAuthenticationRequestData,
            AuthenticationResponseData,
        >,
    >
    for RouterData<
        PostAuthentication,
        ConnectorPostAuthenticationRequestData,
        AuthenticationResponseData,
    >
{
    type Error = Error;

    fn try_from(
        item: ResponseRouterData<
            PostAuthentication,
            AciPaymentsResponse,
            ConnectorPostAuthenticationRequestData,
            AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = if item.response.result.is_failure() {
            Err(item.response.result.decline(
                &item.response.id,
                item.response.result_details.as_ref(),
                item.http_code,
            ))
        } else {
            let tds = item.response.three_d_secure.as_ref();
            Ok(AuthenticationResponseData::PostAuthNResponse {
                trans_status: if ResultClass::of(&item.response.result.code)
                    == Some(ResultClass::Success)
                {
                    enums::TransactionStatus::Success
                } else {
                    enums::TransactionStatus::Failure
                },
                authentication_value: tds.and_then(AciThreeDSecureResponse::authentication_value),
                eci: tds.and_then(|t| t.eci.clone()),
                challenge_cancel: None,
                challenge_code_reason: None,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}
