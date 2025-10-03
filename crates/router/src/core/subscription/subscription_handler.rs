use std::str::FromStr;

use api_models::{
    enums as api_enums,
    subscription::{self as subscription_types, SubscriptionResponse, SubscriptionStatus},
};
use common_enums::connector_enums;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    router_response_types::subscriptions as subscription_response_types,
};
use masking::Secret;

use super::errors;
use crate::{core::subscription::invoice_handler::InvoiceHandler, routes::SessionState};

pub struct SubscriptionHandler<'a> {
    pub state: &'a SessionState,
    pub merchant_context: &'a MerchantContext,
    pub profile: hyperswitch_domain_models::business_profile::Profile,
}

impl<'a> SubscriptionHandler<'a> {
    pub fn new(
        state: &'a SessionState,
        merchant_context: &'a MerchantContext,
        profile: hyperswitch_domain_models::business_profile::Profile,
    ) -> Self {
        Self {
            state,
            merchant_context,
            profile,
        }
    }

    /// Helper function to create a subscription entry in the database.
    pub async fn create_subscription_entry(
        &self,
        subscription_id: common_utils::id_type::SubscriptionId,
        customer_id: &common_utils::id_type::CustomerId,
        billing_processor: connector_enums::Connector,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        merchant_reference_id: Option<String>,
    ) -> errors::RouterResult<SubscriptionWithHandler<'_>> {
        let store = self.state.store.clone();
        let db = store.as_ref();

        let mut subscription = SubscriptionNew::new(
            subscription_id,
            SubscriptionStatus::Created.to_string(),
            Some(billing_processor.to_string()),
            None,
            Some(merchant_connector_id),
            None,
            None,
            self.merchant_context
                .get_merchant_account()
                .get_id()
                .clone(),
            customer_id.clone(),
            None,
            self.profile.get_id().clone(),
            merchant_reference_id,
        );

        subscription.generate_and_set_client_secret();

        let new_subscription = db
            .insert_subscription_entry(subscription)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("subscriptions: unable to insert subscription entry to database")?;

        Ok(SubscriptionWithHandler {
            handler: self,
            subscription: new_subscription,
            profile: self.profile.clone(),
            merchant_account: self.merchant_context.get_merchant_account().clone(),
        })
    }

    /// Helper function to find and validate customer.
    pub async fn find_customer(
        state: &SessionState,
        merchant_context: &MerchantContext,
        customer_id: &common_utils::id_type::CustomerId,
    ) -> errors::RouterResult<hyperswitch_domain_models::customer::Customer> {
        let key_manager_state = &(state).into();
        let merchant_key_store = merchant_context.get_merchant_key_store();
        let merchant_id = merchant_context.get_merchant_account().get_id();

        state
            .store
            .find_customer_by_customer_id_merchant_id(
                key_manager_state,
                customer_id,
                merchant_id,
                merchant_key_store,
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::CustomerNotFound)
            .attach_printable("subscriptions: unable to fetch customer from database")
    }

    /// Helper function to find business profile.
    pub async fn find_business_profile(
        state: &SessionState,
        merchant_context: &MerchantContext,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> errors::RouterResult<hyperswitch_domain_models::business_profile::Profile> {
        let key_manager_state = &(state).into();
        let merchant_key_store = merchant_context.get_merchant_key_store();

        state
            .store
            .find_business_profile_by_profile_id(key_manager_state, merchant_key_store, profile_id)
            .await
            .change_context(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_string(),
            })
    }

    pub async fn find_subscription(
        &self,
        subscription_id: common_utils::id_type::SubscriptionId,
    ) -> errors::RouterResult<SubscriptionWithHandler<'_>> {
        let subscription = self
            .state
            .store
            .find_by_merchant_id_subscription_id(
                self.merchant_context.get_merchant_account().get_id(),
                subscription_id.get_string_repr().to_string().clone(),
            )
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!(
                    "subscription not found for id: {}",
                    subscription_id.get_string_repr()
                ),
            })?;

        Ok(SubscriptionWithHandler {
            handler: self,
            subscription,
            profile: self.profile.clone(),
            merchant_account: self.merchant_context.get_merchant_account().clone(),
        })
    }
}
pub struct SubscriptionWithHandler<'a> {
    pub handler: &'a SubscriptionHandler<'a>,
    pub subscription: diesel_models::subscription::Subscription,
    pub profile: hyperswitch_domain_models::business_profile::Profile,
    pub merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
}

impl SubscriptionWithHandler<'_> {
    pub fn generate_response(
        &self,
        invoice: &diesel_models::invoice::Invoice,
        payment_response: &subscription_types::PaymentResponseData,
        status: subscription_response_types::SubscriptionStatus,
    ) -> errors::RouterResult<subscription_types::ConfirmSubscriptionResponse> {
        Ok(subscription_types::ConfirmSubscriptionResponse {
            id: self.subscription.id.clone(),
            merchant_reference_id: self.subscription.merchant_reference_id.clone(),
            status: SubscriptionStatus::from(status),
            plan_id: None,
            profile_id: self.subscription.profile_id.to_owned(),
            payment: Some(payment_response.clone()),
            customer_id: Some(self.subscription.customer_id.clone()),
            price_id: None,
            coupon: None,
            billing_processor_subscription_id: self.subscription.connector_subscription_id.clone(),
            invoice: Some(subscription_types::Invoice {
                id: invoice.id.clone(),
                subscription_id: invoice.subscription_id.clone(),
                merchant_id: invoice.merchant_id.clone(),
                profile_id: invoice.profile_id.clone(),
                merchant_connector_id: invoice.merchant_connector_id.clone(),
                payment_intent_id: invoice.payment_intent_id.clone(),
                payment_method_id: invoice.payment_method_id.clone(),
                customer_id: invoice.customer_id.clone(),
                amount: invoice.amount,
                currency: api_enums::Currency::from_str(invoice.currency.as_str())
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "currency",
                    })
                    .attach_printable(format!(
                        "unable to parse currency name {currency:?}",
                        currency = invoice.currency
                    ))?,
                status: invoice.status.clone(),
            }),
        })
    }

    pub fn to_subscription_response(&self) -> SubscriptionResponse {
        SubscriptionResponse::new(
            self.subscription.id.clone(),
            self.subscription.merchant_reference_id.clone(),
            SubscriptionStatus::from_str(&self.subscription.status)
                .unwrap_or(SubscriptionStatus::Created),
            None,
            self.subscription.profile_id.to_owned(),
            self.subscription.merchant_id.to_owned(),
            self.subscription.client_secret.clone().map(Secret::new),
            self.subscription.customer_id.clone(),
        )
    }

    pub async fn update_subscription(
        &mut self,
        subscription_update: diesel_models::subscription::SubscriptionUpdate,
    ) -> errors::RouterResult<()> {
        let db = self.handler.state.store.as_ref();
        let updated_subscription = db
            .update_subscription_entry(
                self.handler
                    .merchant_context
                    .get_merchant_account()
                    .get_id(),
                self.subscription.id.get_string_repr().to_string(),
                subscription_update,
            )
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Subscription Update".to_string(),
            })
            .attach_printable("subscriptions: unable to update subscription entry in database")?;

        self.subscription = updated_subscription;

        Ok(())
    }

    pub fn get_invoice_handler(&self) -> InvoiceHandler {
        InvoiceHandler {
            subscription: self.subscription.clone(),
            merchant_account: self.merchant_account.clone(),
            profile: self.profile.clone(),
        }
    }
}
