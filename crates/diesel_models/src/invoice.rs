use common_utils::pii::SecretSerdeValue;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::invoices;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = invoices)]
pub struct InvoiceNew {
    pub invoice_id: String,
    pub subscription_id: Option<String>,
    pub connector_subscription_id: Option<String>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub payment_intent_id: String,
    pub payment_method_id: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub amount: i32,
    pub currency: String,
    pub status: String,
    pub provider_name: String,
    pub metadata: Option<SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(
    table_name = invoices,
    primary_key(invoice_id),
    check_for_backend(diesel::pg::Pg)
)]
pub struct Invoice {
    pub invoice_id: String,
    pub subscription_id: Option<String>,
    pub connector_subscription_id: Option<String>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub payment_intent_id: String,
    pub payment_method_id: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub amount: i32,
    pub currency: String,
    pub status: String,
    pub provider_name: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, Deserialize)]
#[diesel(table_name = invoices)]
pub struct InvoiceUpdate {
    pub status: Option<String>,
    pub payment_method_id: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
}

impl InvoiceNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        invoice_id: String,
        subscription_id: Option<String>,
        connector_subscription_id: Option<String>,
        merchant_id: common_utils::id_type::MerchantId,
        profile_id: common_utils::id_type::ProfileId,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        payment_intent_id: String,
        payment_method_id: Option<String>,
        customer_id: common_utils::id_type::CustomerId,
        amount: i32,
        currency: String,
        status: String,
        provider_name: String,
        metadata: Option<SecretSerdeValue>,
    ) -> Self {
        let now = common_utils::date_time::now();
        Self {
            invoice_id,
            subscription_id,
            connector_subscription_id,
            merchant_id,
            profile_id,
            merchant_connector_id,
            payment_intent_id,
            payment_method_id,
            customer_id,
            amount,
            currency,
            status,
            provider_name,
            metadata,
            created_at: now,
            modified_at: now,
        }
    }
}

impl InvoiceUpdate {
    pub fn new(payment_method_id: Option<String>, status: Option<String>) -> Self {
        Self {
            payment_method_id,
            status,
            modified_at: common_utils::date_time::now(),
        }
    }
}
