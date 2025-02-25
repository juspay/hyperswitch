use common_enums::ApiVersion;
use common_utils::{encryption::Encryption, pii, types::Description};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

#[cfg(all(feature = "v2", feature = "customer_v2"))]
use crate::enums::DeleteStatus;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::schema::customers;
#[cfg(all(feature = "v2", feature = "customer_v2"))]
use crate::schema_v2::customers;

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[derive(
    Clone, Debug, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize, Insertable,
)]
#[diesel(table_name = customers)]
pub struct CustomerNew {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<Description>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub address_id: Option<String>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl CustomerNew {
    pub fn update_storage_scheme(&mut self, storage_scheme: common_enums::MerchantStorageScheme) {
        self.updated_by = Some(storage_scheme.to_string());
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl From<CustomerNew> for Customer {
    fn from(customer_new: CustomerNew) -> Self {
        Self {
            customer_id: customer_new.customer_id,
            merchant_id: customer_new.merchant_id,
            name: customer_new.name,
            email: customer_new.email,
            phone: customer_new.phone,
            phone_country_code: customer_new.phone_country_code,
            description: customer_new.description,
            created_at: customer_new.created_at,
            metadata: customer_new.metadata,
            connector_customer: customer_new.connector_customer,
            modified_at: customer_new.modified_at,
            address_id: customer_new.address_id,
            default_payment_method_id: None,
            updated_by: customer_new.updated_by,
            version: customer_new.version,
        }
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[derive(
    Clone, Debug, Insertable, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers, primary_key(id))]
pub struct CustomerNew {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<ConnectorCustomerMap>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub merchant_reference_id: Option<common_utils::id_type::CustomerId>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub status: DeleteStatus,
    pub id: common_utils::id_type::GlobalCustomerId,
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl CustomerNew {
    pub fn update_storage_scheme(&mut self, storage_scheme: common_enums::MerchantStorageScheme) {
        self.updated_by = Some(storage_scheme.to_string());
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl From<CustomerNew> for Customer {
    fn from(customer_new: CustomerNew) -> Self {
        Self {
            merchant_id: customer_new.merchant_id,
            name: customer_new.name,
            email: customer_new.email,
            phone: customer_new.phone,
            phone_country_code: customer_new.phone_country_code,
            description: customer_new.description,
            created_at: customer_new.created_at,
            metadata: customer_new.metadata,
            connector_customer: customer_new.connector_customer,
            modified_at: customer_new.modified_at,
            default_payment_method_id: None,
            updated_by: customer_new.updated_by,
            merchant_reference_id: customer_new.merchant_reference_id,
            default_billing_address: customer_new.default_billing_address,
            default_shipping_address: customer_new.default_shipping_address,
            id: customer_new.id,
            version: customer_new.version,
            status: customer_new.status,
        }
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers, primary_key(customer_id, merchant_id), check_for_backend(diesel::pg::Pg))]
pub struct Customer {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<String>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = customers, primary_key(id))]
pub struct Customer {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub phone_country_code: Option<String>,
    pub description: Option<Description>,
    pub created_at: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer: Option<ConnectorCustomerMap>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub merchant_reference_id: Option<common_utils::id_type::CustomerId>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub status: DeleteStatus,
    pub id: common_utils::id_type::GlobalCustomerId,
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[derive(
    Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers)]
pub struct CustomerUpdateInternal {
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<Description>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<pii::SecretSerdeValue>,
    pub address_id: Option<String>,
    pub default_payment_method_id: Option<Option<String>>,
    pub updated_by: Option<String>,
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl CustomerUpdateInternal {
    pub fn apply_changeset(self, source: Customer) -> Customer {
        let Self {
            name,
            email,
            phone,
            description,
            phone_country_code,
            metadata,
            connector_customer,
            address_id,
            default_payment_method_id,
            ..
        } = self;

        Customer {
            name: name.map_or(source.name, Some),
            email: email.map_or(source.email, Some),
            phone: phone.map_or(source.phone, Some),
            description: description.map_or(source.description, Some),
            phone_country_code: phone_country_code.map_or(source.phone_country_code, Some),
            metadata: metadata.map_or(source.metadata, Some),
            modified_at: common_utils::date_time::now(),
            connector_customer: connector_customer.map_or(source.connector_customer, Some),
            address_id: address_id.map_or(source.address_id, Some),
            default_payment_method_id: default_payment_method_id
                .flatten()
                .map_or(source.default_payment_method_id, Some),
            ..source
        }
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[derive(
    Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, serde::Deserialize, serde::Serialize,
)]
#[diesel(table_name = customers)]
pub struct CustomerUpdateInternal {
    pub name: Option<Encryption>,
    pub email: Option<Encryption>,
    pub phone: Option<Encryption>,
    pub description: Option<Description>,
    pub phone_country_code: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: PrimitiveDateTime,
    pub connector_customer: Option<ConnectorCustomerMap>,
    pub default_payment_method_id: Option<Option<common_utils::id_type::GlobalPaymentMethodId>>,
    pub updated_by: Option<String>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub status: Option<DeleteStatus>,
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl CustomerUpdateInternal {
    pub fn apply_changeset(self, source: Customer) -> Customer {
        let Self {
            name,
            email,
            phone,
            description,
            phone_country_code,
            metadata,
            connector_customer,
            default_payment_method_id,
            default_billing_address,
            default_shipping_address,
            status,
            ..
        } = self;

        Customer {
            name: name.map_or(source.name, Some),
            email: email.map_or(source.email, Some),
            phone: phone.map_or(source.phone, Some),
            description: description.map_or(source.description, Some),
            phone_country_code: phone_country_code.map_or(source.phone_country_code, Some),
            metadata: metadata.map_or(source.metadata, Some),
            modified_at: common_utils::date_time::now(),
            connector_customer: connector_customer.map_or(source.connector_customer, Some),
            default_payment_method_id: default_payment_method_id
                .flatten()
                .map_or(source.default_payment_method_id, Some),
            default_billing_address: default_billing_address
                .map_or(source.default_billing_address, Some),
            default_shipping_address: default_shipping_address
                .map_or(source.default_shipping_address, Some),
            status: status.unwrap_or(source.status),
            ..source
        }
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
#[serde(transparent)]
pub struct ConnectorCustomerMap(
    std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>,
);

#[cfg(all(feature = "v2", feature = "customer_v2"))]
common_utils::impl_to_sql_from_sql_json!(ConnectorCustomerMap);

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl std::ops::Deref for ConnectorCustomerMap {
    type Target =
        std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl std::ops::DerefMut for ConnectorCustomerMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
