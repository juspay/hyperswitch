//! Capture (`CP`): settle all or part of a preauthorization.

use common_enums::enums;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    router_data::{ErrorResponse, RouterData},
    router_request_types::ResponseId,
    router_response_types::PaymentsResponseData,
    types::PaymentsCaptureRouterData,
};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connectors::aci::transformers::{
        request::{AciPaymentType, TransactionDetails},
        response::AciResponseResultDetails,
        result::{ResultClass, ResultCode},
        AciAuthType, AciRouterData, Error,
    },
    types::ResponseRouterData,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCaptureRequest {
    #[serde(flatten)]
    txn_details: TransactionDetails,
}

impl TryFrom<&AciRouterData<&PaymentsCaptureRouterData>> for AciCaptureRequest {
    type Error = Error;

    fn try_from(item: &AciRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            txn_details: TransactionDetails {
                entity_id: auth.entity_id,
                amount: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
                payment_type: AciPaymentType::Capture,
            },
        })
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCaptureResponse {
    id: String,
    referenced_id: String,
    payment_type: AciPaymentType,
    amount: StringMajorUnit,
    currency: String,
    descriptor: String,
    result: ResultCode,
    result_details: Option<AciResponseResultDetails>,
    build_number: String,
    timestamp: String,
    ndc: Secret<String>,
    source: Option<Secret<String>>,
    payment_method: Option<String>,
    short_id: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, AciCaptureResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, AciCaptureResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let ResponseRouterData {
            response,
            data,
            http_code,
        } = item;
        let status = match response.result.class()? {
            ResultClass::Success => enums::AttemptStatus::Charged,
            ResultClass::Pending => enums::AttemptStatus::Pending,
            ResultClass::Failure => enums::AttemptStatus::Failure,
        };
        let outcome = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                attempt_status: Some(status),
                ..response
                    .result
                    .decline(&response.id, response.result_details.as_ref(), http_code)
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: Some(response.referenced_id.clone()),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                payment_account_reference: None,
            })
        };
        Ok(Self {
            status,
            response: outcome,
            reference_id: Some(response.referenced_id),
            ..data
        })
    }
}
