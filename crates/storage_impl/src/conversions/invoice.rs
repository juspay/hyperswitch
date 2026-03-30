//! Conversion implementations for Invoice and InvoiceUpdate

use common_utils::{
    date_time,
    encryption::Encryption,
    errors::{CustomResult, ValidationError},
    types::keymanager::{Identifier, KeyManagerState},
};
use hyperswitch_domain_models::invoice::{Invoice, InvoiceUpdate};
use hyperswitch_masking::Secret;

use crate::behaviour::Conversion;

#[async_trait::async_trait]
impl Conversion for Invoice {
    type DstType = diesel_models::invoice::Invoice;
    type NewDstType = diesel_models::invoice::InvoiceNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let now = date_time::now();
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

#[async_trait::async_trait]
impl Conversion for InvoiceUpdate {
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
