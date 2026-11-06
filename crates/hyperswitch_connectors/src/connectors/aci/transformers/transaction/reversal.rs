//! Reversal (`RV`): void a preauthorization. Answered by the payment response.

use hyperswitch_domain_models::types::PaymentsCancelRouterData;
use hyperswitch_masking::Secret;
use serde::Serialize;

use crate::connectors::aci::transformers::{request::AciPaymentType, AciAuthType, Error};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCancelRequest {
    entity_id: Secret<String>,
    payment_type: AciPaymentType,
}

impl TryFrom<&PaymentsCancelRouterData> for AciCancelRequest {
    type Error = Error;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            entity_id: auth.entity_id,
            payment_type: AciPaymentType::Reversal,
        })
    }
}
