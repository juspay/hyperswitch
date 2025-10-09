use common_utils::pii;

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodSession {
    pub id: common_utils::id_type::GlobalPaymentMethodSessionId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub billing: Option<common_utils::encryption::Encryption>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub tokenization_data: Option<pii::SecretSerdeValue>,
    pub return_url: Option<common_utils::types::Url>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expires_at: time::PrimitiveDateTime,
    pub associated_payment_methods: Option<Vec<String>>,
    pub associated_payment: Option<common_utils::id_type::GlobalPaymentId>,
    pub associated_token_id: Option<common_utils::id_type::GlobalTokenId>,
}

#[cfg(feature = "v2")]
impl PaymentMethodSession {
    pub fn apply_changeset(self, update_session: PaymentMethodsSessionUpdateInternal) -> Self {
        let Self {
            id,
            customer_id,
            billing,
            psp_tokenization,
            network_tokenization,
            tokenization_data,
            expires_at,
            return_url,
            associated_payment_methods,
            associated_payment,
            associated_token_id,
        } = self;

        Self {
            id,
            customer_id,
            billing: update_session.billing.or(billing),
            psp_tokenization: update_session.psp_tokenization.or(psp_tokenization),
            network_tokenization: update_session.network_tokenization.or(network_tokenization),
            tokenization_data: update_session.tokenzation_data.or(tokenization_data),
            expires_at,
            return_url,
            associated_payment_methods,
            associated_payment,
            associated_token_id,
        }
    }
}

#[cfg(feature = "v2")]
pub struct PaymentMethodsSessionUpdateInternal {
    pub billing: Option<common_utils::encryption::Encryption>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    pub tokenzation_data: Option<masking::Secret<serde_json::Value>>,
}
