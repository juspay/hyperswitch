use common_utils::{
    id_type::GenerateId,
    errors::{CustomResult, ParsingError, ValidationError},
    pii::SecretSerdeValue,
    types::keymanager::{Identifier, KeyManagerState},
    types::MinorUnit,
};
use error_stack::ResultExt;
use masking::Secret;
use std::str::FromStr;
use utoipa::ToSchema;
use crate::merchant_key_store::MerchantKeyStore;


#[derive(Debug, Clone, serde::Serialize, ToSchema)]
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
}

pub enum InvoiceStatus {
    InvoiceCreated,
    PaymentPending,
    PaymentPendingTimeout,
    PaymentSucceeded,
    PaymentFailed,
    PaymentCanceled,
    InvoicePaid,
    ManualReview,
}

#[async_trait::async_trait]

impl super::behaviour::Conversion for Invoice {
    type DstType = diesel_models::invoice::Invoice;
    type NewDstType = diesel_models::invoice::InvoiceNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let now = common_utils::date_time::now();
        Ok(diesel_models::invoice::Invoice {
            id: self.id,
            subscription_id: self.subscription_id,
            merchant_id: self.merchant_id,
            profile_id: self.profile_id,
            merchant_connector_id: self.merchant_connector_id,
            payment_intent_id: self.payment_intent_id,
            payment_method_id: self.payment_method_id,
            customer_id: self.customer_id,
            amount: self.amount,
            currency: self.currency.to_string(),
            status: self.status,
            provider_name: common_enums::connector_enums::Connector::Adyen, // Placeholder connector
            metadata: None,
            created_at: now,
            modified_at: now,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: item.id,
            subscription_id: item.subscription_id,
            merchant_id: item.merchant_id,
            profile_id: item.profile_id,
            merchant_connector_id: item.merchant_connector_id,
            payment_intent_id: item.payment_intent_id,
            payment_method_id: item.payment_method_id,
            customer_id: item.customer_id,
            amount: item.amount,
            // currency: Currency::from_str(&item.currency).change_context(
            //     ValidationError::InvalidValue {
            //         message: "Invalid currency value".to_string(),
            //     },
            // )?,
            currency: item.currency,
            status: item.status,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        let invoice_status = common_enums::connector_enums::InvoiceStatus::from_str(&self.status)
            .change_context(ValidationError::InvalidValue {
            message: "Invalid invoice status".to_string(),
        })?;

        Ok(diesel_models::invoice::InvoiceNew::new(
            self.subscription_id,
            self.merchant_id,
            self.profile_id,
            self.merchant_connector_id,
            self.payment_intent_id,
            self.payment_method_id,
            self.customer_id,
            self.amount,
            self.currency.to_string(),
            invoice_status,
            common_enums::connector_enums::Connector::Adyen, // Placeholder connector
            None,
        ))
    }
}

// Type conversions for Invoice

/// Convert from API model `Invoice` to domain model `Invoice`
impl From<api_models::subscription::Invoice> for Invoice {
    fn from(api_invoice: api_models::subscription::Invoice) -> Self {
        Self {
            id: api_invoice.id,
            subscription_id: api_invoice.subscription_id,
            merchant_id: api_invoice.merchant_id,
            profile_id: api_invoice.profile_id,
            merchant_connector_id: api_invoice.merchant_connector_id,
            payment_intent_id: api_invoice.payment_intent_id,
            payment_method_id: api_invoice.payment_method_id,
            customer_id: api_invoice.customer_id,
            amount: api_invoice.amount,
            currency: api_invoice.currency.to_string(),
            status: api_invoice.status,
        }
    }
}

// /// Convert from domain model `Invoice` to API model `Invoice`
// impl From<Invoice> for api_models::subscription::Invoice {
//     fn from(domain_invoice: Invoice) -> Self {
//         Self {
//             id: domain_invoice.id,
//             subscription_id: domain_invoice.subscription_id,
//             merchant_id: domain_invoice.merchant_id,
//             profile_id: domain_invoice.profile_id,
//             merchant_connector_id: domain_invoice.merchant_connector_id,
//             payment_intent_id: domain_invoice.payment_intent_id,
//             payment_method_id: domain_invoice.payment_method_id,
//             customer_id: domain_invoice.customer_id,
//             amount: domain_invoice.amount,
//             currency: domain_invoice.currency.into(),
//             status: domain_invoice.status,
//         }
//     }
// }

impl Invoice {
    /// Convert domain invoice with context to InvoiceNew for database operations
    pub fn to_invoice_new(
        subscription_id: common_utils::id_type::SubscriptionId,
        merchant_id: common_utils::id_type::MerchantId,
        profile_id: common_utils::id_type::ProfileId,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
        payment_method_id: Option<String>,
        customer_id: common_utils::id_type::CustomerId,
        amount: MinorUnit,
        currency: String,
        status: common_enums::connector_enums::InvoiceStatus,
        provider_name: common_enums::connector_enums::Connector,
        metadata: Option<SecretSerdeValue>,
    ) -> diesel_models::invoice::InvoiceNew {
        diesel_models::invoice::InvoiceNew::new(
            subscription_id,
            merchant_id,
            profile_id,
            merchant_connector_id,
            payment_intent_id,
            payment_method_id,
            customer_id,
            amount,
            currency,
            status,
            provider_name,
            metadata,
        )
    }
    pub fn to_invoice(
        subscription_id: common_utils::id_type::SubscriptionId,
        merchant_id: common_utils::id_type::MerchantId,
        profile_id: common_utils::id_type::ProfileId,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
        payment_method_id: Option<String>,
        customer_id: common_utils::id_type::CustomerId,
        amount: MinorUnit,
        currency: String,
        status: common_enums::connector_enums::InvoiceStatus,
        _provider_name: common_enums::connector_enums::Connector,
        _metadata: Option<SecretSerdeValue>,
    ) -> Invoice {
        // let now = common_utils::date_time::now();
        Invoice{
            id:common_utils::id_type::InvoiceId::generate(),
            subscription_id:subscription_id,
            merchant_id:merchant_id,
            profile_id:profile_id,
            merchant_connector_id:merchant_connector_id,
            payment_intent_id:payment_intent_id,
            payment_method_id:payment_method_id,
            customer_id:customer_id,
            amount:amount,
            currency:currency.to_string(),
            status:status.to_string(),
        }
    }

    /// Convert from database invoice model to domain invoice
    pub fn from_invoice_db(invoice: diesel_models::invoice::Invoice) -> Result<Self, ParsingError> {
        Ok(Self {
            id: invoice.id,
            subscription_id: invoice.subscription_id,
            merchant_id: invoice.merchant_id,
            profile_id: invoice.profile_id,
            merchant_connector_id: invoice.merchant_connector_id,
            payment_intent_id: invoice.payment_intent_id,
            payment_method_id: invoice.payment_method_id,
            customer_id: invoice.customer_id,
            amount: invoice.amount,
            currency: invoice.currency,
            // currency: Currency::from_str(&invoice.currency)
            //     .map_err(|_| ParsingError::UnknownError)?,
            status: invoice.status,
        })
    }
}

#[async_trait::async_trait]
pub trait InvoiceInterface {
    type Error;
    async fn insert_invoice_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_new: Invoice,
    ) -> CustomResult<Invoice, Self::Error>;

    async fn find_invoice_by_invoice_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_id: String,
    ) -> CustomResult<Invoice, Self::Error>;

    async fn update_invoice_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        invoice_id: String,
        data: InvoiceUpdate,
    ) -> CustomResult<Invoice, Self::Error>;
}

pub struct InvoiceUpdate {
    pub status: Option<String>,
    pub payment_method_id: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
}
#[async_trait::async_trait]
impl super::behaviour::Conversion for InvoiceUpdate {
    type DstType = diesel_models::invoice::InvoiceUpdate;
    type NewDstType = diesel_models::invoice::InvoiceUpdate;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::invoice::InvoiceUpdate {
            status: self.status,
            payment_method_id: self.payment_method_id,
            modified_at: self.modified_at,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            status: item.status,
            payment_method_id: item.payment_method_id,
            modified_at: item.modified_at,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::invoice::InvoiceUpdate {
            status: self.status,
            payment_method_id: self.payment_method_id,
            modified_at: self.modified_at,
        })
    }
}