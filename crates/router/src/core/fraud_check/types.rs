#[cfg(all(feature = "payouts", feature = "v1"))]
use std::collections::HashSet;

use api_models::{
    enums as api_enums,
    enums::{PaymentMethod, PaymentMethodType},
    payments::Amount,
    payouts::PayoutMethodData,
    refunds::RefundResponse,
};
use common_enums::FrmSuggestion;
use common_utils::pii::SecretSerdeValue;
use hyperswitch_domain_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
pub use hyperswitch_domain_models::{
    address::Address as PayoutAddress,
    customer::Customer,
    merchant_connector_account::MerchantConnectorAccount,
    payouts::payout_attempt::PayoutAttempt,
    router_request_types::fraud_check::{
        Address, Destination, FrmFulfillmentRequest, FulfillmentStatus, Fulfillments, Product,
    },
    types::OrderDetailsWithAmount,
};
use hyperswitch_masking::Serialize;
use serde::Deserialize;
use utoipa::ToSchema;

use super::operation::BoxedFraudCheckOperation;
use crate::types::{
    api::routing::FrmRoutingAlgorithm,
    domain::MerchantAccount,
    storage::{
        enums::{self as storage_enums, FraudCheckStatus},
        fraud_check::FraudCheck,
    },
    PaymentAddress,
};

#[derive(Clone, Default, Debug)]
pub struct PaymentIntentCore {
    pub payment_id: common_utils::id_type::PaymentId,
}

#[derive(Clone, Debug)]
pub struct PaymentAttemptCore {
    pub attempt_id: String,
    pub payment_details: Option<PaymentDetails>,
    pub amount: Amount,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaymentDetails {
    pub amount: i64,
    pub currency: Option<storage_enums::Currency>,
    pub payment_method: Option<PaymentMethod>,
    pub payment_method_type: Option<PaymentMethodType>,
    pub refund_transaction_id: Option<String>,
}
#[derive(Clone, Default, Debug)]
pub struct FrmMerchantAccount {
    pub merchant_id: common_utils::id_type::MerchantId,
}

#[derive(Clone, Debug)]
pub struct FrmData {
    pub payment_intent: PaymentIntent,
    pub payment_attempt: PaymentAttempt,
    pub merchant_account: MerchantAccount,
    pub fraud_check: FraudCheck,
    pub address: PaymentAddress,
    pub connector_details: ConnectorDetailsCore,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub refund: Option<RefundResponse>,
    pub frm_metadata: Option<SecretSerdeValue>,
}

#[derive(Debug)]
pub struct FrmInfo<F, D> {
    pub fraud_check_operation: BoxedFraudCheckOperation<F, D>,
    pub frm_data: Option<FrmData>,
    pub suggested_action: Option<FrmSuggestion>,
}

#[derive(Clone, Debug)]
pub struct ConnectorDetailsCore {
    pub connector_name: String,
    pub profile_id: common_utils::id_type::ProfileId,
}
#[derive(Clone)]
pub struct PaymentToFrmData {
    pub amount: Amount,
    pub payment_intent: PaymentIntent,
    pub payment_attempt: PaymentAttempt,
    pub merchant_account: MerchantAccount,
    pub address: PaymentAddress,
    pub connector_details: ConnectorDetailsCore,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub frm_metadata: Option<SecretSerdeValue>,
}

#[derive(Clone, Debug)]
pub struct PayoutFrmData {
    pub fraud_check: FraudCheck,
    pub amount: common_utils::types::MinorUnit,
    pub currency: storage_enums::Currency,
    pub payout_attempt: PayoutAttempt,
    pub payout_method_data: Option<PayoutMethodData>,
    pub customer_details: Option<Customer>,
    pub billing_address: Option<PayoutAddress>,
}

impl PayoutFrmData {
    pub fn should_cancel_payout(&self) -> bool {
        matches!(self.fraud_check.frm_status, FraudCheckStatus::Fraud)
    }

    pub fn get_frm_outcome(&self) -> PayoutFrmOutcome {
        if self.should_cancel_payout() {
            PayoutFrmOutcome::Blocked {
                error_code: self.fraud_check.frm_status.to_string(),
                error_message: self
                    .fraud_check
                    .frm_reason
                    .as_ref()
                    .map(|reason| match reason {
                        serde_json::Value::String(value) => value.clone(),
                        other => other.to_string(),
                    })
                    .or(self.fraud_check.frm_error.clone()),
            }
        } else {
            PayoutFrmOutcome::Continue
        }
    }
}

#[cfg(all(feature = "payouts", feature = "v1"))]
#[derive(Debug, Clone)]
pub enum PayoutFrmApplicability {
    Applicable {
        connectors: HashSet<api_enums::Connector>,
        frm_routing_algorithm: FrmRoutingAlgorithm,
    },
    NotApplicable,
}

#[cfg(all(feature = "payouts", feature = "v1"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayoutFrmOutcome {
    Continue,
    Blocked {
        error_code: String,
        error_message: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrmConfigsObject {
    pub frm_enabled_pm: Option<PaymentMethod>,
    pub frm_enabled_gateway: Option<api_models::enums::Connector>,
    pub frm_preferred_flow_type: api_enums::FrmPreferredFlowTypes,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydApiRequest {
    ///unique order_id for the order_details in the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub order_id: String,
    ///denotes the status of the fulfillment... can be one of PARTIAL, COMPLETE, REPLACEMENT, CANCELED
    #[schema(value_type = Option<FulfillmentStatus>, example = "COMPLETE")]
    pub fulfillment_status: Option<FulfillmentStatus>,
    ///contains details of the fulfillment
    #[schema(value_type = Vec<Fulfillments>)]
    pub fulfillments: Vec<Fulfillments>,
}

#[derive(Debug, ToSchema, Clone, Serialize)]
pub struct FrmFulfillmentResponse {
    ///unique order_id for the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub order_id: String,
    ///shipment_ids used in the fulfillment overall...also data from previous fulfillments for the same transactions/order is sent
    #[schema(example = r#"["ship_101", "ship_102"]"#)]
    pub shipment_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct FrmFulfillmentSignifydApiResponse {
    ///unique order_id for the transaction
    #[schema(max_length = 255, example = "pay_qiYfHcDou1ycIaxVXKHF")]
    pub order_id: String,
    ///shipment_ids used in the fulfillment overall...also data from previous fulfillments for the same transactions/order is sent
    #[schema(example = r#"["ship_101","ship_102"]"#)]
    pub shipment_ids: Vec<String>,
}

pub const CANCEL_INITIATED: &str = "Cancel Initiated with the processor";
