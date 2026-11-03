//! Refund (`RF`): return all or part of a captured payment.

use common_enums::enums;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    router_data::ErrorResponse, router_response_types::RefundsResponseData,
    types::RefundsRouterData,
};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connectors::aci::transformers::{
        request::AciPaymentType,
        response::AciResponseResultDetails,
        result::{ResultClass, ResultCode},
        AciAuthType, AciRouterData, Error,
    },
    types::RefundsResponseRouterData,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRefundRequest {
    amount: StringMajorUnit,
    currency: String,
    payment_type: AciPaymentType,
    entity_id: Secret<String>,
}

impl<F> TryFrom<&AciRouterData<&RefundsRouterData<F>>> for AciRefundRequest {
    type Error = Error;
    fn try_from(item: &AciRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            amount: item.amount.to_owned(),
            currency: item.router_data.request.currency.to_string(),
            payment_type: AciPaymentType::Refund,
            entity_id: auth.entity_id,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentReference {
    #[serde(rename = "referenceId")]
    reference_id: Option<String>,
    #[serde(rename = "type")]
    reference_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRefundResponse {
    id: String,
    ndc: String,
    timestamp: String,
    build_number: String,
    result: ResultCode,
    result_details: Option<AciResponseResultDetails>,
    references: Option<Vec<AciPaymentReference>>,
}

impl<F> TryFrom<RefundsResponseRouterData<F, AciRefundResponse>> for RefundsRouterData<F> {
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<F, AciRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let response = item.response;
        let refund_status = match response.result.class()? {
            ResultClass::Success => enums::RefundStatus::Success,
            ResultClass::Pending => enums::RefundStatus::Pending,
            ResultClass::Failure => enums::RefundStatus::Failure,
        };
        let outcome = if refund_status == enums::RefundStatus::Failure {
            Err(ErrorResponse {
                connector_response_reference_id: None,
                ..response.result.decline(
                    &response.id,
                    response.result_details.as_ref(),
                    item.http_code,
                )
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: response.id,
                refund_status,
            })
        };
        Ok(Self {
            response: outcome,
            ..item.data
        })
    }
}
