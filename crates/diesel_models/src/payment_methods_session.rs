#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodsSession {
    pub id: common_utils::id_type::GlobalPaymentMethodSessionId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub billing: Option<common_utils::encryption::Encryption>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokeinzation: Option<common_types::payment_methods::NetworkTokenization>,
    pub return_url: Option<common_utils::types::Url>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expires_at: time::PrimitiveDateTime,
    pub associated_payment_methods: Option<Vec<common_utils::id_type::GlobalPaymentMethodId>>,
    pub associated_payment: Option<common_utils::id_type::GlobalPaymentId>,
}
