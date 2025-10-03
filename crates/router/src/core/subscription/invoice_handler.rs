use api_models::{
    enums as api_enums,
    subscription::{self as subscription_types},
};
use common_enums::connector_enums;
use common_utils::{pii, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_response_types::subscriptions as subscription_response_types;
use masking::{PeekInterface, Secret};

use super::errors;
use crate::{
    core::subscription::payments_api_client, routes::SessionState, types::storage as storage_types,
    workflows::invoice_sync as invoice_sync_workflow,
};

pub struct InvoiceHandler {
    pub subscription: diesel_models::subscription::Subscription,
    pub merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
    pub profile: hyperswitch_domain_models::business_profile::Profile,
}

#[allow(clippy::todo)]
impl InvoiceHandler {
    pub fn new(
        subscription: diesel_models::subscription::Subscription,
        merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
        profile: hyperswitch_domain_models::business_profile::Profile,
    ) -> Self {
        Self {
            subscription,
            merchant_account,
            profile,
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
    ) -> errors::RouterResult<diesel_models::invoice::Invoice> {
        let invoice_new = diesel_models::invoice::InvoiceNew::new(
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
        );

        let invoice = state
            .store
            .insert_invoice_entry(invoice_new)
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
        payment_method_id: Option<Secret<String>>,
        status: connector_enums::InvoiceStatus,
    ) -> errors::RouterResult<diesel_models::invoice::Invoice> {
        let update_invoice = diesel_models::invoice::InvoiceUpdate::new(
            payment_method_id.as_ref().map(|id| id.peek()).cloned(),
            Some(status),
        );
        state
            .store
            .update_invoice_entry(invoice_id.get_string_repr().to_string(), update_invoice)
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
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        let payment_details = &request.payment_details;
        let payment_request = subscription_types::CreatePaymentsRequestData {
            amount: request.amount,
            currency: request.currency,
            customer_id: Some(self.subscription.customer_id.clone()),
            billing: request.billing.clone(),
            shipping: request.shipping.clone(),
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
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
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
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        let payment_details = &request.payment_details;
        let payment_request = subscription_types::CreateAndConfirmPaymentsRequestData {
            amount,
            currency,
            confirm: true,
            customer_id: Some(self.subscription.customer_id.clone()),
            billing: request.billing.clone(),
            shipping: request.shipping.clone(),
            setup_future_usage: payment_details.setup_future_usage,
            return_url: payment_details.return_url.clone(),
            capture_method: payment_details.capture_method,
            authentication_type: payment_details.authentication_type,
            payment_method: payment_details.payment_method,
            payment_method_type: payment_details.payment_method_type,
            payment_method_data: payment_details.payment_method_data.clone(),
            customer_acceptance: payment_details.customer_acceptance.clone(),
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
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        let payment_details = &request.payment_details;
        let cit_payment_request = subscription_types::ConfirmPaymentsRequestData {
            billing: request.billing.clone(),
            shipping: request.shipping.clone(),
            payment_method: payment_details.payment_method,
            payment_method_type: payment_details.payment_method_type,
            payment_method_data: payment_details.payment_method_data.clone(),
            customer_acceptance: payment_details.customer_acceptance.clone(),
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
    ) -> errors::RouterResult<diesel_models::invoice::Invoice> {
        state
            .store
            .get_latest_invoice_for_subscription(self.subscription.id.get_string_repr().to_string())
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
    ) -> errors::RouterResult<diesel_models::invoice::Invoice> {
        state
            .store
            .find_invoice_by_invoice_id(invoice_id.get_string_repr().to_string())
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Get Invoice by ID".to_string(),
            })
            .attach_printable("invoices: unable to get invoice by id from database")
    }

    pub async fn create_invoice_sync_job(
        &self,
        state: &SessionState,
        invoice: &diesel_models::invoice::Invoice,
        connector_invoice_id: String,
        connector_name: connector_enums::Connector,
    ) -> errors::RouterResult<()> {
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
}
