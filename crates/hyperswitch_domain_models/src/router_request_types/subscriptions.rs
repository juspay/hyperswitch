use common_enums::enums;

use crate::connector_endpoints;


#[derive(Debug, Clone)]
pub struct SubscriptionsRecordBackRequest {
    pub merchant_reference_id: String,
    pub amount: common_utils::types::MinorUnit,
    pub currency: enums::Currency,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub attempt_status: common_enums::AttemptStatus,
    pub connector_transaction_id: Option<common_utils::types::ConnectorTransactionId>,
    pub connector_params: connector_endpoints::ConnectorParams,
}
