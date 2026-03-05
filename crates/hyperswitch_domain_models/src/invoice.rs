use common_utils::{
    errors::{CustomResult, ValidationError},
    id_type::GenerateId,
    pii::SecretSerdeValue,
    types::{
        keymanager::{Identifier, KeyManagerState},
        MinorUnit,
    },
};
use masking::{PeekInterface, Secret};
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
    pub status: common_enums::connector_enums::InvoiceStatus,
    pub provider_name: common_enums::connector_enums::Connector,
    pub metadata: Option<SecretSerdeValue>,
    pub connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
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
            provider_name: self.provider_name,
            metadata: None,
            created_at: now,
            modified_at: now,
            connector_invoice_id: self.connector_invoice_id,
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
            currency: item.currency,
            status: item.status,
            provider_name: item.provider_name,
            metadata: item.metadata,
            connector_invoice_id: item.connector_invoice_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
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
            self.status,
            self.provider_name,
            None,
            self.connector_invoice_id,
        ))
    }
}

impl Invoice {
    #[allow(clippy::too_many_arguments)]
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
        provider_name: common_enums::connector_enums::Connector,
        metadata: Option<SecretSerdeValue>,
        connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
    ) -> Self {
        Self {
            id: common_utils::id_type::InvoiceId::generate(),
            subscription_id,
            merchant_id,
            profile_id,
            merchant_connector_id,
            payment_intent_id,
            payment_method_id,
            customer_id,
            amount,
            currency: currency.to_string(),
            status,
            provider_name,
            metadata,
            connector_invoice_id,
        }
    }
}

#[async_trait::async_trait]
pub trait InvoiceInterface {
    type Error;
    async fn insert_invoice_entry(
        &self,
        key_store: &MerchantKeyStore,
        invoice_new: Invoice,
    ) -> CustomResult<Invoice, Self::Error>;

    async fn find_invoice_by_invoice_id(
        &self,
        key_store: &MerchantKeyStore,
        invoice_id: String,
    ) -> CustomResult<Invoice, Self::Error>;

    async fn update_invoice_entry(
        &self,
        key_store: &MerchantKeyStore,
        invoice_id: String,
        data: InvoiceUpdate,
    ) -> CustomResult<Invoice, Self::Error>;

    async fn get_latest_invoice_for_subscription(
        &self,
        key_store: &MerchantKeyStore,
        subscription_id: String,
    ) -> CustomResult<Invoice, Self::Error>;

    async fn find_invoice_by_subscription_id_connector_invoice_id(
        &self,
        key_store: &MerchantKeyStore,
        subscription_id: String,
        connector_invoice_id: common_utils::id_type::InvoiceId,
    ) -> CustomResult<Option<Invoice>, Self::Error>;
}

pub struct InvoiceUpdate {
    pub status: Option<common_enums::connector_enums::InvoiceStatus>,
    pub payment_method_id: Option<String>,
    pub connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
    pub modified_at: time::PrimitiveDateTime,
    pub payment_intent_id: Option<common_utils::id_type::PaymentId>,
    pub amount: Option<MinorUnit>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AmountAndCurrencyUpdate {
    pub amount: MinorUnit,
    pub currency: String,
}

#[derive(Debug, Clone)]
pub struct ConnectorAndStatusUpdate {
    pub connector_invoice_id: common_utils::id_type::InvoiceId,
    pub status: common_enums::connector_enums::InvoiceStatus,
}

#[derive(Debug, Clone)]
pub struct PaymentAndStatusUpdate {
    pub payment_method_id: Option<Secret<String>>,
    pub payment_intent_id: Option<common_utils::id_type::PaymentId>,
    pub status: common_enums::connector_enums::InvoiceStatus,
    pub connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
}

/// Enum-based invoice update request for different scenarios
#[derive(Debug, Clone)]
pub enum InvoiceUpdateRequest {
    /// Update amount and currency
    Amount(AmountAndCurrencyUpdate),
    /// Update connector invoice ID and status
    Connector(ConnectorAndStatusUpdate),
    /// Update payment details along with status
    PaymentStatus(PaymentAndStatusUpdate),
}

impl InvoiceUpdateRequest {
    /// Create an amount and currency update request
    pub fn update_amount_and_currency(amount: MinorUnit, currency: String) -> Self {
        Self::Amount(AmountAndCurrencyUpdate { amount, currency })
    }

    /// Create a connector invoice ID and status update request
    pub fn update_connector_and_status(
        connector_invoice_id: common_utils::id_type::InvoiceId,
        status: common_enums::connector_enums::InvoiceStatus,
    ) -> Self {
        Self::Connector(ConnectorAndStatusUpdate {
            connector_invoice_id,
            status,
        })
    }

    /// Create a combined payment and status update request
    pub fn update_payment_and_status(
        payment_method_id: Option<Secret<String>>,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
        status: common_enums::connector_enums::InvoiceStatus,
        connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
    ) -> Self {
        Self::PaymentStatus(PaymentAndStatusUpdate {
            payment_method_id,
            payment_intent_id,
            status,
            connector_invoice_id,
        })
    }
}

impl From<InvoiceUpdateRequest> for InvoiceUpdate {
    fn from(request: InvoiceUpdateRequest) -> Self {
        let now = common_utils::date_time::now();

        match request {
            InvoiceUpdateRequest::Amount(update) => Self {
                status: None,
                payment_method_id: None,
                connector_invoice_id: None,
                modified_at: now,
                payment_intent_id: None,
                amount: Some(update.amount),
                currency: Some(update.currency),
            },
            InvoiceUpdateRequest::Connector(update) => Self {
                status: Some(update.status),
                payment_method_id: None,
                connector_invoice_id: Some(update.connector_invoice_id),
                modified_at: now,
                payment_intent_id: None,
                amount: None,
                currency: None,
            },
            InvoiceUpdateRequest::PaymentStatus(update) => Self {
                status: Some(update.status),
                payment_method_id: update
                    .payment_method_id
                    .as_ref()
                    .map(|id| id.peek())
                    .cloned(),
                connector_invoice_id: update.connector_invoice_id,
                modified_at: now,
                payment_intent_id: update.payment_intent_id,
                amount: None,
                currency: None,
            },
        }
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for InvoiceUpdate {
    type DstType = diesel_models::invoice::InvoiceUpdate;
    type NewDstType = diesel_models::invoice::InvoiceUpdate;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::invoice::InvoiceUpdate {
            status: self.status,
            payment_method_id: self.payment_method_id,
            connector_invoice_id: self.connector_invoice_id,
            modified_at: self.modified_at,
            payment_intent_id: self.payment_intent_id,
            amount: self.amount,
            currency: self.currency,
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
            connector_invoice_id: item.connector_invoice_id,
            modified_at: item.modified_at,
            payment_intent_id: item.payment_intent_id,
            amount: item.amount,
            currency: item.currency,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::invoice::InvoiceUpdate {
            status: self.status,
            payment_method_id: self.payment_method_id,
            connector_invoice_id: self.connector_invoice_id,
            modified_at: self.modified_at,
            payment_intent_id: self.payment_intent_id,
            amount: self.amount,
            currency: self.currency,
        })
    }
}

impl InvoiceUpdate {
    pub fn new(
        payment_method_id: Option<String>,
        status: Option<common_enums::connector_enums::InvoiceStatus>,
        connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
        amount: Option<MinorUnit>,
        currency: Option<String>,
    ) -> Self {
        Self {
            status,
            payment_method_id,
            connector_invoice_id,
            modified_at: common_utils::date_time::now(),
            payment_intent_id,
            amount,
            currency,
        }
    }
}
