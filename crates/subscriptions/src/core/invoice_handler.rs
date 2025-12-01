use api_models::{
    enums as api_enums,
    mandates::RecurringDetails,
    subscription::{self as subscription_types},
};
use common_enums::connector_enums;
use common_utils::{pii, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_response_types::subscriptions as subscription_response_types;
use masking::PeekInterface;

use super::errors;
use crate::{
    core::payments_api_client, state::SubscriptionState as SessionState,
    types::storage as storage_types, workflows::invoice_sync as invoice_sync_workflow,
};

pub struct InvoiceHandler {
    pub subscription: hyperswitch_domain_models::subscription::Subscription,
    pub merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
    pub profile: hyperswitch_domain_models::business_profile::Profile,
    pub merchant_key_store: hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
}

#[allow(clippy::todo)]
impl InvoiceHandler {
    pub fn new(
        subscription: hyperswitch_domain_models::subscription::Subscription,
        merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
        profile: hyperswitch_domain_models::business_profile::Profile,
        merchant_key_store: hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
    ) -> Self {
        Self {
            subscription,
            merchant_account,
            profile,
            merchant_key_store,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn create_invoice_entry(
        &self,
        state: &SessionState,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
        amount: MinorUnit,
        currency: common_enums::Currency,
        status: connector_enums::InvoiceStatus,
        provider_name: connector_enums::Connector,
        metadata: Option<pii::SecretSerdeValue>,
        connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
    ) -> errors::SubscriptionResult<hyperswitch_domain_models::invoice::Invoice> {
        let invoice_new = hyperswitch_domain_models::invoice::Invoice::to_invoice(
            self.subscription.id.to_owned(),
            self.subscription.merchant_id.to_owned(),
            self.subscription.profile_id.to_owned(),
            merchant_connector_id,
            payment_intent_id,
            self.subscription.payment_method_id.clone(),
            self.subscription.customer_id.to_owned(),
            amount,
            currency.to_string(),
            status,
            provider_name,
            metadata,
            connector_invoice_id,
        );

        let invoice = state
            .store
            .insert_invoice_entry(&self.merchant_key_store, invoice_new)
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Create Invoice".to_string(),
            })
            .attach_printable("invoices: unable to insert invoice entry to database")?;

        Ok(invoice)
    }

    pub async fn update_invoice(
        &self,
        state: &SessionState,
        invoice_id: common_utils::id_type::InvoiceId,
        update_request: hyperswitch_domain_models::invoice::InvoiceUpdateRequest,
    ) -> errors::SubscriptionResult<hyperswitch_domain_models::invoice::Invoice> {
        let update_invoice: hyperswitch_domain_models::invoice::InvoiceUpdate =
            update_request.into();
        state
            .store
            .update_invoice_entry(
                &self.merchant_key_store,
                invoice_id.get_string_repr().to_string(),
                update_invoice,
            )
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Invoice Update".to_string(),
            })
            .attach_printable("invoices: unable to update invoice entry in database")
    }

    pub fn get_amount_and_currency(
        request: (Option<MinorUnit>, Option<api_enums::Currency>),
        invoice_details: Option<subscription_response_types::SubscriptionInvoiceData>,
    ) -> (MinorUnit, api_enums::Currency) {
        // Use request amount and currency if provided, else fallback to invoice details from connector response
        request.0.zip(request.1).unwrap_or(
            invoice_details
                .clone()
                .map(|invoice| (invoice.total, invoice.currency_code))
                .unwrap_or((MinorUnit::new(0), api_enums::Currency::default())),
        ) // Default to 0 and a default currency if not provided
    }

    pub async fn create_payment_with_confirm_false(
        &self,
        state: &SessionState,
        request: &subscription_types::CreateSubscriptionRequest,
        amount: MinorUnit,
        currency: api_enums::Currency,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let payment_details = &request.payment_details;
        let payment_request = subscription_types::CreatePaymentsRequestData {
            amount,
            currency,
            customer_id: Some(self.subscription.customer_id.clone()),
            billing: request.billing.clone(),
            shipping: request.shipping.clone(),
            profile_id: Some(self.profile.get_id().clone()),
            setup_future_usage: payment_details.setup_future_usage,
            return_url: Some(payment_details.return_url.clone()),
            capture_method: payment_details.capture_method,
            authentication_type: payment_details.authentication_type,
        };
        payments_api_client::PaymentsApiClient::create_cit_payment(
            state,
            payment_request,
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
    }

    pub async fn get_payment_details(
        &self,
        state: &SessionState,
        payment_id: common_utils::id_type::PaymentId,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        payments_api_client::PaymentsApiClient::sync_payment(
            state,
            payment_id.get_string_repr().to_string(),
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
    }

    pub async fn create_and_confirm_payment(
        &self,
        state: &SessionState,
        request: &subscription_types::CreateAndConfirmSubscriptionRequest,
        amount: MinorUnit,
        currency: common_enums::Currency,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let payment_details = &request.payment_details;
        let payment_request = subscription_types::CreateAndConfirmPaymentsRequestData {
            amount,
            currency,
            confirm: true,
            customer_id: Some(self.subscription.customer_id.clone()),
            billing: request.get_billing_address(),
            shipping: request.shipping.clone(),
            profile_id: Some(self.profile.get_id().clone()),
            setup_future_usage: payment_details
                .payment_method_id
                .is_none()
                .then_some(payment_details.setup_future_usage)
                .flatten(),
            return_url: payment_details.return_url.clone(),
            capture_method: payment_details.capture_method,
            authentication_type: payment_details.authentication_type,
            payment_method: payment_details.payment_method,
            payment_method_type: payment_details.payment_method_type,
            payment_method_data: payment_details.payment_method_data.clone(),
            customer_acceptance: payment_details.customer_acceptance.clone(),
            payment_type: payment_details.payment_type,
            recurring_details: payment_details
                .payment_method_id
                .as_ref()
                .map(|id| RecurringDetails::PaymentMethodId(id.peek().clone())),
            off_session: Some(payment_details.payment_method_id.is_some()),
        };
        payments_api_client::PaymentsApiClient::create_and_confirm_payment(
            state,
            payment_request,
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
    }

    pub async fn confirm_payment(
        &self,
        state: &SessionState,
        payment_id: common_utils::id_type::PaymentId,
        request: &subscription_types::ConfirmSubscriptionRequest,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let payment_details = &request.payment_details;
        let cit_payment_request = subscription_types::ConfirmPaymentsRequestData {
            billing: request.get_billing_address(),
            shipping: request.payment_details.shipping.clone(),
            profile_id: Some(self.profile.get_id().clone()),
            payment_method: payment_details.payment_method,
            payment_method_type: payment_details.payment_method_type,
            payment_method_data: payment_details.payment_method_data.clone(),
            customer_acceptance: payment_details.customer_acceptance.clone(),
            payment_type: payment_details.payment_type,
            payment_token: payment_details.payment_token.clone(),
        };
        payments_api_client::PaymentsApiClient::confirm_payment(
            state,
            cit_payment_request,
            payment_id.get_string_repr().to_string(),
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
    }

    pub async fn get_latest_invoice(
        &self,
        state: &SessionState,
    ) -> errors::SubscriptionResult<hyperswitch_domain_models::invoice::Invoice> {
        state
            .store
            .get_latest_invoice_for_subscription(
                &self.merchant_key_store,
                self.subscription.id.get_string_repr().to_string(),
            )
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Get Latest Invoice".to_string(),
            })
            .attach_printable("invoices: unable to get latest invoice from database")
    }

    pub async fn get_invoice_by_id(
        &self,
        state: &SessionState,
        invoice_id: common_utils::id_type::InvoiceId,
    ) -> errors::SubscriptionResult<hyperswitch_domain_models::invoice::Invoice> {
        state
            .store
            .find_invoice_by_invoice_id(
                &self.merchant_key_store,
                invoice_id.get_string_repr().to_string(),
            )
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Get Invoice by ID".to_string(),
            })
            .attach_printable("invoices: unable to get invoice by id from database")
    }

    pub async fn find_invoice_by_subscription_id_connector_invoice_id(
        &self,
        state: &SessionState,
        subscription_id: common_utils::id_type::SubscriptionId,
        connector_invoice_id: common_utils::id_type::InvoiceId,
    ) -> errors::SubscriptionResult<Option<hyperswitch_domain_models::invoice::Invoice>> {
        state
            .store
            .find_invoice_by_subscription_id_connector_invoice_id(
                &self.merchant_key_store,
                subscription_id.get_string_repr().to_string(),
                connector_invoice_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Get Invoice by Subscription ID and Connector Invoice ID".to_string(),
            })
            .attach_printable("invoices: unable to get invoice by subscription id and connector invoice id from database")
    }

    pub async fn create_invoice_sync_job(
        &self,
        state: &SessionState,
        invoice: &hyperswitch_domain_models::invoice::Invoice,
        connector_invoice_id: Option<common_utils::id_type::InvoiceId>,
        connector_name: connector_enums::Connector,
    ) -> errors::SubscriptionResult<()> {
        let request = storage_types::invoice_sync::InvoiceSyncRequest::new(
            self.subscription.id.to_owned(),
            invoice.id.to_owned(),
            self.subscription.merchant_id.to_owned(),
            self.subscription.profile_id.to_owned(),
            self.subscription.customer_id.to_owned(),
            connector_invoice_id,
            connector_name,
        );

        invoice_sync_workflow::create_invoice_sync_job(state, request)
            .await
            .attach_printable("invoices: unable to create invoice sync job in database")?;
        Ok(())
    }

    pub async fn create_mit_payment(
        &self,
        state: &SessionState,
        amount: MinorUnit,
        currency: common_enums::Currency,
        payment_method_id: &str,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let mit_payment_request = subscription_types::CreateMitPaymentRequestData {
            amount,
            currency,
            confirm: true,
            customer_id: Some(self.subscription.customer_id.clone()),
            recurring_details: Some(RecurringDetails::PaymentMethodId(
                payment_method_id.to_owned(),
            )),
            off_session: Some(true),
            profile_id: Some(self.profile.get_id().clone()),
        };

        payments_api_client::PaymentsApiClient::create_mit_payment(
            state,
            mit_payment_request,
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
    }

    pub async fn update_payment(
        &self,
        state: &SessionState,
        amount: MinorUnit,
        currency: common_enums::Currency,
        payment_id: common_utils::id_type::PaymentId,
    ) -> errors::SubscriptionResult<subscription_types::PaymentResponseData> {
        let payment_update_request = subscription_types::CreatePaymentsRequestData {
            amount,
            currency,
            customer_id: None,
            billing: None,
            shipping: None,
            profile_id: None,
            setup_future_usage: None,
            return_url: None,
            capture_method: None,
            authentication_type: None,
        };

        payments_api_client::PaymentsApiClient::update_payment(
            state,
            payment_update_request,
            payment_id.get_string_repr().to_string(),
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
    }
}
