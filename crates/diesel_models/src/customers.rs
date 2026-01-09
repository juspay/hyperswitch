use common_enums::ApiVersion;
use common_utils::{encryption::Encryption, pii, types::Description};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

#[cfg(feature = "v1")]
use crate::schema::customers;
#[cfg(feature = "v2")]
use crate::{
    diesel_impl::RequiredFromNullableWithDefault, enums::DeleteStatus, schema_v2::customers,
};

#[cfg(feature = "v1")]
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
    pub tax_registration_id: Option<Encryption>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
}

#[cfg(feature = "v1")]
impl CustomerNew {
    pub fn update_storage_scheme(&mut self, storage_scheme: common_enums::MerchantStorageScheme) {
        self.updated_by = Some(storage_scheme.to_string());
    }
}

#[cfg(feature = "v1")]
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
            tax_registration_id: customer_new.tax_registration_id,
            created_by: customer_new.created_by,
            last_modified_by: customer_new.last_modified_by,
        }
    }
}

#[cfg(feature = "v2")]
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
    pub connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub tax_registration_id: Option<Encryption>,
    pub merchant_reference_id: Option<common_utils::id_type::CustomerId>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub status: DeleteStatus,
    pub id: common_utils::id_type::GlobalCustomerId,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
}

#[cfg(feature = "v2")]
impl CustomerNew {
    pub fn update_storage_scheme(&mut self, storage_scheme: common_enums::MerchantStorageScheme) {
        self.updated_by = Some(storage_scheme.to_string());
    }
}

#[cfg(feature = "v2")]
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
            tax_registration_id: customer_new.tax_registration_id,
            merchant_reference_id: customer_new.merchant_reference_id,
            default_billing_address: customer_new.default_billing_address,
            default_shipping_address: customer_new.default_shipping_address,
            id: customer_new.id,
            version: customer_new.version,
            status: customer_new.status,
            created_by: customer_new.created_by,
            last_modified_by: customer_new.last_modified_by,
        }
    }
}

#[cfg(feature = "v1")]
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
    pub tax_registration_id: Option<Encryption>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
}

#[cfg(feature = "v2")]
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
    pub connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
    pub modified_at: PrimitiveDateTime,
    pub default_payment_method_id: Option<common_utils::id_type::GlobalPaymentMethodId>,
    pub updated_by: Option<String>,
    pub version: ApiVersion,
    pub tax_registration_id: Option<Encryption>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub merchant_reference_id: Option<common_utils::id_type::CustomerId>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    #[diesel(deserialize_as = RequiredFromNullableWithDefault<DeleteStatus>)]
    pub status: DeleteStatus,
    pub id: common_utils::id_type::GlobalCustomerId,
}

#[cfg(feature = "v1")]
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
    pub tax_registration_id: Option<Encryption>,
    pub last_modified_by: Option<String>,
}

#[cfg(feature = "v1")]
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
            tax_registration_id,
            last_modified_by,
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
            tax_registration_id: tax_registration_id.map_or(source.tax_registration_id, Some),
            last_modified_by: last_modified_by.or(source.last_modified_by),
            ..source
        }
    }
}

#[cfg(feature = "v2")]
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
    pub connector_customer: Option<common_types::customers::ConnectorCustomerMap>,
    pub default_payment_method_id: Option<Option<common_utils::id_type::GlobalPaymentMethodId>>,
    pub updated_by: Option<String>,
    pub default_billing_address: Option<Encryption>,
    pub default_shipping_address: Option<Encryption>,
    pub status: Option<DeleteStatus>,
    pub tax_registration_id: Option<Encryption>,
    pub last_modified_by: Option<String>,
}

#[cfg(feature = "v2")]
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
            tax_registration_id,
            last_modified_by,
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
            tax_registration_id: tax_registration_id.map_or(source.tax_registration_id, Some),
            last_modified_by: last_modified_by.or(source.last_modified_by),
            ..source
        }
    }
}
