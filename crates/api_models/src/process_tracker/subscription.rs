use common_enums::connector_enums::Connector;
use common_utils::id_type;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionWorkflowTrackingData {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub payment_method_id: Option<String>,
    pub subscription_id: id_type::SubscriptionId,
    pub invoice_id: id_type::InvoiceId,
    pub amount: common_utils::types::MinorUnit,
    pub currency: common_enums::Currency,
    pub customer_id: id_type::CustomerId,
    pub connector_name: Connector,
    pub billing_connector_mca_id: id_type::MerchantConnectorAccountId,
}
