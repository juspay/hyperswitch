use common_utils::id_type;
use diesel::{AsExpression, FromSqlRow};

#[derive(
    Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
/// Represents the data associated with a callback mapper.
pub enum CallbackMapperData {
    /// data variant used while processing the network token webhook
    NetworkTokenWebhook {
        /// Merchant id associated with the network token requestor reference id
        merchant_id: id_type::MerchantId,
        /// Payment Method id associated with the network token requestor reference id
        payment_method_id: String,
        /// Customer id associated with the network token requestor reference id
        customer_id: id_type::CustomerId,
    },
}

impl CallbackMapperData {
    /// Retrieves the details of the network token webhook type from callback mapper data.
    pub fn get_network_token_webhook_details(
        &self,
    ) -> (id_type::MerchantId, String, id_type::CustomerId) {
        match self {
            Self::NetworkTokenWebhook {
                merchant_id,
                payment_method_id,
                customer_id,
            } => (
                merchant_id.clone(),
                payment_method_id.clone(),
                customer_id.clone(),
            ),
        }
    }
}

common_utils::impl_to_sql_from_sql_json!(CallbackMapperData);
