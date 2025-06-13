use common_utils::{id_type};
use diesel::{AsExpression, FromSqlRow};

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
/// Represents the data associated with a callback mapper.
pub enum CallbackMapperData {
    /// data variant used while processing the network token webhook
    NetworkTokenWebhook {
        /// Merchant id assiociated with the network token requestor reference id
        merchant_id: id_type::MerchantId,
        /// Payment Method id assiociated with the network token requestor reference id
        payment_method_id: String,
        /// Customer id assiociated with the network token requestor reference id
        customer_id: id_type::CustomerId,
    },
}

common_utils::impl_to_sql_from_sql_json!(CallbackMapperData);