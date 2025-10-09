use common_enums::connector_enums::{Connector, InvoiceStatus};
use common_utils::{id_type::GenerateId, pii::SecretSerdeValue, types::MinorUnit};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::invoice;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = invoice, check_for_backend(diesel::pg::Pg))]
pub struct InvoiceNew {
    pub id: common_utils::id_type::InvoiceId,
    pub subscription_id: common_utils::id_type::SubscriptionId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub payment_intent_id: Option<common_utils::id_type::PaymentId>,
    pub payment_method_id: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub amount: MinorUnit,
    pub currency: String,
    pub status: String,
    pub provider_name: Connector,
    pub metadata: Option<SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_invoice_id: Option<String>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(
    table_name = invoice,
    primary_key(id),
    check_for_backend(diesel::pg::Pg)
)]
pub struct Invoice {
    pub id: common_utils::id_type::InvoiceId,
    pub subscription_id: common_utils::id_type::SubscriptionId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub payment_intent_id: Option<common_utils::id_type::PaymentId>,
    pub payment_method_id: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub amount: MinorUnit,
    pub currency: String,
    pub status: String,
    pub provider_name: Connector,
    pub metadata: Option<SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_invoice_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, Deserialize)]
#[diesel(table_name = invoice)]
pub struct InvoiceUpdate {
    pub status: Option<String>,
    pub payment_method_id: Option<String>,
    pub connector_invoice_id: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
    pub payment_intent_id: Option<common_utils::id_type::PaymentId>,
}

impl InvoiceNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subscription_id: common_utils::id_type::SubscriptionId,
        merchant_id: common_utils::id_type::MerchantId,
        profile_id: common_utils::id_type::ProfileId,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
        payment_method_id: Option<String>,
        customer_id: common_utils::id_type::CustomerId,
        amount: MinorUnit,
        currency: String,
        status: InvoiceStatus,
        provider_name: Connector,
        metadata: Option<SecretSerdeValue>,
        connector_invoice_id: Option<String>,
    ) -> Self {
        let id = common_utils::id_type::InvoiceId::generate();
        let now = common_utils::date_time::now();
        Self {
            id,
            subscription_id,
            merchant_id,
            profile_id,
            merchant_connector_id,
            payment_intent_id,
            payment_method_id,
            customer_id,
            amount,
            currency,
            status: status.to_string(),
            provider_name,
            metadata,
            created_at: now,
            modified_at: now,
            connector_invoice_id,
        }
    }
}

impl InvoiceUpdate {
    pub fn new(
        payment_method_id: Option<String>,
        status: Option<InvoiceStatus>,
        connector_invoice_id: Option<String>,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
    ) -> Self {
        Self {
            payment_method_id,
            status: status.map(|status| status.to_string()),
            connector_invoice_id,
            payment_intent_id,
            modified_at: common_utils::date_time::now(),
        }
    }
}
