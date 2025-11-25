use std::str::FromStr;

use api_models::{
    enums as api_enums,
    subscription::{self as subscription_types, SubscriptionResponse},
};
use common_enums::connector_enums;
use common_utils::{consts, ext_traits::OptionExt};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    customer, merchant_connector_account,
    platform::Platform,
    router_response_types::{self, subscriptions as subscription_response_types},
    subscription::{Subscription, SubscriptionStatus},
};
use masking::Secret;

use super::errors;
use crate::{
    core::invoice_handler::InvoiceHandler,
    errors::CustomResult,
    helpers::{ForeignTryFrom, StorageErrorExt},
    state::SubscriptionState as SessionState,
};

pub struct SubscriptionHandler<'a> {
    pub state: &'a SessionState,
    pub platform: &'a Platform,
}

impl<'a> SubscriptionHandler<'a> {
    pub fn new(state: &'a SessionState, platform: &'a Platform) -> Self {
        Self { state, platform }
    }

    #[allow(clippy::too_many_arguments)]
    /// Helper function to create a subscription entry in the database.
    pub async fn create_subscription_entry(
        &self,
        subscription_id: common_utils::id_type::SubscriptionId,
        customer_id: &common_utils::id_type::CustomerId,
        billing_processor: connector_enums::Connector,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        merchant_reference_id: Option<String>,
        profile: &hyperswitch_domain_models::business_profile::Profile,
        plan_id: Option<String>,
        item_price_id: Option<String>,
    ) -> errors::SubscriptionResult<SubscriptionWithHandler<'_>> {
        let store = self.state.store.clone();
        let db = store.as_ref();

        let mut subscription = Subscription {
            id: subscription_id,
            status: SubscriptionStatus::Created.to_string(),
            billing_processor: Some(billing_processor.to_string()),
            payment_method_id: None,
            merchant_connector_id: Some(merchant_connector_id),
            client_secret: None,
            connector_subscription_id: None,
            merchant_id: self.platform.get_processor().get_account().get_id().clone(),
            customer_id: customer_id.clone(),
            metadata: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            profile_id: profile.get_id().clone(),
            merchant_reference_id,
            plan_id,
            item_price_id,
        };

        subscription.generate_and_set_client_secret();

        let merchant_key_store = self.platform.get_processor().get_key_store();
        let new_subscription = db
            .insert_subscription_entry(merchant_key_store, subscription)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("subscriptions: unable to insert subscription entry to database")?;

        Ok(SubscriptionWithHandler {
            handler: self,
            subscription: new_subscription,
            merchant_account: self.platform.get_processor().get_account().clone(),
        })
    }

    /// Helper function to find and validate customer.
    pub async fn find_customer(
        state: &SessionState,
        platform: &Platform,
        customer_id: &common_utils::id_type::CustomerId,
    ) -> errors::SubscriptionResult<customer::Customer> {
        let merchant_key_store = platform.get_processor().get_key_store();
        let merchant_id = platform.get_processor().get_account().get_id();

        state
            .store
            .find_customer_by_customer_id_merchant_id(
                customer_id,
                merchant_id,
                merchant_key_store,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::CustomerNotFound)
            .attach_printable("subscriptions: unable to fetch customer from database")
    }
    pub async fn update_connector_customer_id_in_customer(
        state: &SessionState,
        platform: &Platform,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        customer: &customer::Customer,
        customer_create_response: Option<router_response_types::ConnectorCustomerResponseData>,
    ) -> errors::SubscriptionResult<customer::Customer> {
        match customer_create_response {
            Some(customer_response) => {
                match customer::update_connector_customer_in_customers(
                    merchant_connector_id.get_string_repr(),
                    Some(customer),
                    Some(customer_response.connector_customer_id),
                )
                .await
                {
                    Some(customer_update) => {
                        Self::update_customer(state, platform, customer.clone(), customer_update)
                            .await
                            .attach_printable(
                                "Failed to update customer with connector customer ID",
                            )
                    }
                    None => Ok(customer.clone()),
                }
            }
            None => Ok(customer.clone()),
        }
    }

    pub async fn update_customer(
        state: &SessionState,
        platform: &Platform,
        customer: customer::Customer,
        customer_update: customer::CustomerUpdate,
    ) -> errors::SubscriptionResult<customer::Customer> {
        let merchant_key_store = platform.get_processor().get_key_store();
        let merchant_id = platform.get_processor().get_account().get_id();
        let db = state.store.as_ref();

        let updated_customer = db
            .update_customer_by_customer_id_merchant_id(
                customer.customer_id.clone(),
                merchant_id.clone(),
                customer,
                customer_update,
                merchant_key_store,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("subscriptions: unable to update customer entry in database")?;

        Ok(updated_customer)
    }

    /// Helper function to find business profile.
    pub async fn find_business_profile(
        state: &SessionState,
        platform: &Platform,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> errors::SubscriptionResult<hyperswitch_domain_models::business_profile::Profile> {
        let merchant_key_store = platform.get_processor().get_key_store();

        state
            .store
            .find_business_profile_by_profile_id(merchant_key_store, profile_id)
            .await
            .change_context(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_string(),
            })
    }

    pub async fn find_and_validate_subscription(
        &self,
        client_secret: &hyperswitch_domain_models::subscription::ClientSecret,
    ) -> errors::SubscriptionResult<()> {
        let subscription_id = client_secret.get_subscription_id()?;
        let key_store = self.platform.get_processor().get_key_store();

        let subscription = self
            .state
            .store
            .find_by_merchant_id_subscription_id(
                key_store,
                self.platform.get_processor().get_account().get_id(),
                subscription_id.to_string(),
            )
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!("Subscription not found for id: {subscription_id}"),
            })
            .attach_printable("Unable to find subscription")?;

        self.validate_client_secret(client_secret, &subscription)?;

        Ok(())
    }

    pub fn validate_client_secret(
        &self,
        client_secret: &hyperswitch_domain_models::subscription::ClientSecret,
        subscription: &Subscription,
    ) -> errors::SubscriptionResult<()> {
        let stored_client_secret = subscription
            .client_secret
            .clone()
            .get_required_value("client_secret")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret",
            })
            .attach_printable("client secret not found in db")?;

        if client_secret.to_string() != stored_client_secret {
            Err(errors::ApiErrorResponse::ClientSecretInvalid.into())
        } else {
            let current_timestamp = common_utils::date_time::now();
            let session_expiry = subscription
                .created_at
                .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

            if current_timestamp > session_expiry {
                Err(errors::ApiErrorResponse::ClientSecretExpired.into())
            } else {
                Ok(())
            }
        }
    }

    pub async fn find_subscription(
        &self,
        subscription_id: common_utils::id_type::SubscriptionId,
    ) -> errors::SubscriptionResult<SubscriptionWithHandler<'_>> {
        let subscription = self
            .state
            .store
            .find_by_merchant_id_subscription_id(
                self.platform.get_processor().get_key_store(),
                self.platform.get_processor().get_account().get_id(),
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
            merchant_account: self.platform.get_processor().get_account().clone(),
        })
    }
}
pub struct SubscriptionWithHandler<'a> {
    pub handler: &'a SubscriptionHandler<'a>,
    pub subscription: Subscription,
    pub merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
}

impl SubscriptionWithHandler<'_> {
    pub fn generate_response(
        &self,
        invoice: &hyperswitch_domain_models::invoice::Invoice,
        payment_response: &subscription_types::PaymentResponseData,
        status: subscription_response_types::SubscriptionStatus,
    ) -> errors::SubscriptionResult<subscription_types::ConfirmSubscriptionResponse> {
        Ok(subscription_types::ConfirmSubscriptionResponse {
            id: self.subscription.id.clone(),
            merchant_reference_id: self.subscription.merchant_reference_id.clone(),
            status: common_enums::SubscriptionStatus::from(status),
            plan_id: self.subscription.plan_id.clone(),
            profile_id: self.subscription.profile_id.to_owned(),
            payment: Some(payment_response.clone()),
            customer_id: Some(self.subscription.customer_id.clone()),
            item_price_id: self.subscription.item_price_id.clone(),
            coupon: None,
            billing_processor_subscription_id: self.subscription.connector_subscription_id.clone(),
            invoice: Some(subscription_types::Invoice::foreign_try_from(invoice)?),
        })
    }

    pub fn to_subscription_response(
        &self,
        payment: Option<subscription_types::PaymentResponseData>,
        invoice: Option<&hyperswitch_domain_models::invoice::Invoice>,
    ) -> errors::SubscriptionResult<SubscriptionResponse> {
        Ok(SubscriptionResponse::new(
            self.subscription.id.clone(),
            self.subscription.merchant_reference_id.clone(),
            common_enums::SubscriptionStatus::from_str(&self.subscription.status)
                .unwrap_or(common_enums::SubscriptionStatus::Created),
            self.subscription.plan_id.clone(),
            self.subscription.item_price_id.clone(),
            self.subscription.profile_id.to_owned(),
            self.subscription.merchant_id.to_owned(),
            self.subscription.client_secret.clone().map(Secret::new),
            self.subscription.customer_id.clone(),
            payment,
            invoice
                .map(
                    |invoice| -> errors::SubscriptionResult<subscription_types::Invoice> {
                        subscription_types::Invoice::foreign_try_from(invoice)
                    },
                )
                .transpose()?,
        ))
    }

    pub async fn update_subscription(
        &mut self,
        subscription_update: hyperswitch_domain_models::subscription::SubscriptionUpdate,
    ) -> errors::SubscriptionResult<()> {
        let db = self.handler.state.store.as_ref();
        let updated_subscription = db
            .update_subscription_entry(
                self.handler.platform.get_processor().get_key_store(),
                self.handler.platform.get_processor().get_account().get_id(),
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

    pub fn get_invoice_handler(
        &self,
        profile: hyperswitch_domain_models::business_profile::Profile,
    ) -> InvoiceHandler {
        InvoiceHandler {
            subscription: self.subscription.clone(),
            merchant_account: self.merchant_account.clone(),
            profile,
            merchant_key_store: self
                .handler
                .platform
                .get_processor()
                .get_key_store()
                .clone(),
        }
    }
    pub async fn get_mca(
        &mut self,
        connector_name: &str,
    ) -> CustomResult<merchant_connector_account::MerchantConnectorAccount, errors::ApiErrorResponse>
    {
        let db = self.handler.state.store.as_ref();

        match &self.subscription.merchant_connector_id {
            Some(merchant_connector_id) => {
                #[cfg(feature = "v1")]
                {
                    db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                        self.handler.platform.get_processor().get_account().get_id(),
                        merchant_connector_id,
                        self.handler.platform.get_processor().get_key_store(),
                    )
                    .await
                    .to_not_found_response(
                        errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                            id: merchant_connector_id.get_string_repr().to_string(),
                        },
                    )
                }
            }
            None => {
                // Fallback to profile-based lookup when merchant_connector_id is not set
                #[cfg(feature = "v1")]
                {
                    db.find_merchant_connector_account_by_profile_id_connector_name(
                        &self.subscription.profile_id,
                        connector_name,
                        self.handler.platform.get_processor().get_key_store(),
                    )
                    .await
                    .to_not_found_response(
                        errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                            id: format!(
                                "profile_id {} and connector_name {connector_name}",
                                self.subscription.profile_id.get_string_repr()
                            ),
                        },
                    )
                }
            }
        }
    }
}

impl ForeignTryFrom<&hyperswitch_domain_models::invoice::Invoice> for subscription_types::Invoice {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        invoice: &hyperswitch_domain_models::invoice::Invoice,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
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
            billing_processor_invoice_id: invoice
                .connector_invoice_id
                .as_ref()
                .map(|id| id.get_string_repr().to_string()),
        })
    }
}
