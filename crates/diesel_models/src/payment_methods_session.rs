#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodsSession {
    pub id: common_utils::id_type::GlobalPaymentMethodSessionId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub billing: Option<common_utils::encryption::Encryption>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub expires_at: time::PrimitiveDateTime,
}

#[cfg(feature = "v2")]
impl PaymentMethodsSession {
    pub fn apply_changeset(
        self,
        update_session: PaymentMethodsSessionUpdateInternal,
    ) -> PaymentMethodsSession {
        let Self {
            id,
            customer_id,
            billing,
            psp_tokenization,
            network_tokenization,
            expires_at,
        } = self;
        PaymentMethodsSession {
            id,
            customer_id,
            billing: update_session.billing.or(billing),
            psp_tokenization: update_session.psp_tokenization.or(psp_tokenization),
            network_tokenization: update_session.network_tokenization.or(network_tokenization),
            expires_at,
        }
    }
}

#[cfg(feature = "v2")]
pub struct PaymentMethodsSessionUpdateInternal {
    pub billing: Option<common_utils::encryption::Encryption>,
    pub psp_tokenization: Option<common_types::payment_methods::PspTokenization>,
    pub network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
}