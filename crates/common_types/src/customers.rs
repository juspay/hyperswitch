//! Customer related types

/// HashMap containing MerchantConnectorAccountId and corresponding customer id
#[cfg(feature = "v2")]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
#[serde(transparent)]
pub struct ConnectorCustomerMap(
    std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>,
);

#[cfg(feature = "v2")]
impl ConnectorCustomerMap {
    /// Creates a new `ConnectorCustomerMap` from a HashMap
    pub fn new(
        map: std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>,
    ) -> Self {
        Self(map)
    }
}

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(ConnectorCustomerMap);

#[cfg(feature = "v2")]
impl std::ops::Deref for ConnectorCustomerMap {
    type Target =
        std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "v2")]
impl std::ops::DerefMut for ConnectorCustomerMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
