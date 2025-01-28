#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodsSession {
    pub id: common_utils::id_type::GlobalPaymentMethodSessionId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub billing: Option<common_utils::encryption::Encryption>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokeinzation: Option<common_types::payment_methods::NetworkTokenization>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expires_at: time::PrimitiveDateTime,
}
