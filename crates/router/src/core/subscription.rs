use std::str::FromStr;

use api_models::{
    enums as api_enums,
    subscription::{self as subscription_types, SubscriptionResponse, SubscriptionStatus},
};
use common_enums::connector_enums;
use common_utils::{ext_traits::ValueExt, id_type::GenerateId, pii, types::MinorUnit};
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    merchant_context::MerchantContext,
    router_data_v2::flow_common_types::{SubscriptionCreateData, SubscriptionCustomerData},
    router_request_types::{subscriptions as subscription_request_types, ConnectorCustomerData},
    router_response_types::{
        subscriptions as subscription_response_types, ConnectorCustomerResponseData,
        PaymentsResponseData,
    },
};
use masking::{PeekInterface, Secret};

use super::errors::{self, RouterResponse};
use crate::{
    core::payments as payments_core, routes::SessionState, services, types::api as api_types,
};

pub mod payments_api_client;

pub const SUBSCRIPTION_CONNECTOR_ID: &str = "DefaultSubscriptionConnectorId";
pub const SUBSCRIPTION_PAYMENT_ID: &str = "DefaultSubscriptionPaymentId";

pub async fn create_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<SubscriptionResponse> {
    let subscription_id = common_utils::id_type::SubscriptionId::generate();

    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile")?;
    let customer =
        SubscriptionHandler::find_customer(&state, &merchant_context, &request.customer_id)
            .await
            .attach_printable("subscriptions: failed to find customer")?;
    let billing_handler =
        BillingHandler::create(&state, &merchant_context, customer, profile.clone()).await?;

    let subscription_handler = SubscriptionHandler::new(&state, &merchant_context, profile);
    let mut subscription = subscription_handler
        .create_subscription_entry(
            subscription_id,
            &request.customer_id,
            billing_handler.connector_data.connector_name,
            billing_handler.merchant_connector_id.clone(),
            request.merchant_reference_id.clone(),
        )
        .await
        .attach_printable("subscriptions: failed to create subscription entry")?;
    let invoice_handler = subscription.get_invoice_handler();
    let payment = invoice_handler
        .create_payment_with_confirm_false(subscription.handler.state, &request)
        .await
        .attach_printable("subscriptions: failed to create payment")?;
    invoice_handler
        .create_invoice_entry(
            &state,
            billing_handler.merchant_connector_id,
            Some(payment.payment_id.clone()),
            request.amount,
            request.currency,
            connector_enums::InvoiceStatus::InvoiceCreated,
            billing_handler.connector_data.connector_name,
            None,
        )
        .await
        .attach_printable("subscriptions: failed to create invoice")?;

    subscription
        .update_subscription(diesel_models::subscription::SubscriptionUpdate::new(
            payment.payment_method_id.clone(),
            None,
            None,
        ))
        .await
        .attach_printable("subscriptions: failed to update subscription")?;

    Ok(ApplicationResponse::Json(
        subscription.to_subscription_response(),
    ))
}

/// Creates and confirms a subscription in one operation.
/// This method combines the creation and confirmation flow to reduce API calls
pub async fn create_and_confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::CreateAndConfirmSubscriptionRequest,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let subscription_id = common_utils::id_type::SubscriptionId::generate();

    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile")?;
    let customer =
        SubscriptionHandler::find_customer(&state, &merchant_context, &request.customer_id)
            .await
            .attach_printable("subscriptions: failed to find customer")?;

    let billing_handler =
        BillingHandler::create(&state, &merchant_context, customer, profile.clone()).await?;
    let subscription_handler = SubscriptionHandler::new(&state, &merchant_context, profile.clone());
    let mut subs_handler = subscription_handler
        .create_subscription_entry(
            subscription_id.clone(),
            &request.customer_id,
            billing_handler.connector_data.connector_name,
            billing_handler.merchant_connector_id.clone(),
            request.merchant_reference_id.clone(),
        )
        .await
        .attach_printable("subscriptions: failed to create subscription entry")?;
    let invoice_handler = subs_handler.get_invoice_handler();

    let _customer_create_response = billing_handler
        .create_customer_on_connector(
            &state,
            request.customer_id.clone(),
            request.billing.clone(),
            request
                .payment_details
                .payment_method_data
                .clone()
                .and_then(|data| data.payment_method_data),
        )
        .await?;

    let subscription_create_response = billing_handler
        .create_subscription_on_connector(
            &state,
            subs_handler.subscription.clone(),
            request.item_price_id.clone(),
            request.billing.clone(),
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    let (amount, currency) = InvoiceHandler::get_amount_and_currency(
        (request.amount, request.currency),
        invoice_details.clone(),
    );

    let payment_response = invoice_handler
        .create_and_confirm_payment(&state, &request, amount, currency)
        .await?;

    let invoice_entry = invoice_handler
        .create_invoice_entry(
            &state,
            profile.get_billing_processor_id()?,
            Some(payment_response.payment_id.clone()),
            amount,
            currency,
            invoice_details
                .and_then(|invoice| invoice.status)
                .unwrap_or(connector_enums::InvoiceStatus::InvoiceCreated),
            billing_handler.connector_data.connector_name,
            None,
        )
        .await?;

    // invoice_entry
    //     .create_invoice_record_back_job(&payment_response)
    //     .await?;

    subs_handler
        .update_subscription(diesel_models::subscription::SubscriptionUpdate::new(
            payment_response.payment_method_id.clone(),
            Some(SubscriptionStatus::from(subscription_create_response.status).to_string()),
            Some(
                subscription_create_response
                    .subscription_id
                    .get_string_repr()
                    .to_string(),
            ),
        ))
        .await?;

    let response = subs_handler.generate_response(
        &invoice_entry,
        &payment_response,
        subscription_create_response.status,
    )?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    // Find the subscription from database
    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable("subscriptions: failed to find business profile")?;
    let customer =
        SubscriptionHandler::find_customer(&state, &merchant_context, &request.customer_id)
            .await
            .attach_printable("subscriptions: failed to find customer")?;

    let handler = SubscriptionHandler::new(&state, &merchant_context, profile.clone());
    let mut subscription_entry = handler.find_subscription(subscription_id).await?;
    let invoice_handler = subscription_entry.get_invoice_handler();
    let invoice = invoice_handler
        .get_latest_invoice(&state)
        .await
        .attach_printable("subscriptions: failed to get latest invoice")?;
    let payment_response = invoice_handler
        .confirm_payment(
            &state,
            invoice
                .payment_intent_id
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "payment_intent_id",
                })?,
            &request,
        )
        .await?;

    let billing_handler =
        BillingHandler::create(&state, &merchant_context, customer, profile).await?;
    let invoice_handler = subscription_entry.get_invoice_handler();
    let subscription = subscription_entry.subscription.clone();

    let _customer_create_response = billing_handler
        .create_customer_on_connector(
            &state,
            subscription.customer_id.clone(),
            request.billing.clone(),
            request
                .payment_details
                .payment_method_data
                .payment_method_data,
        )
        .await?;

    let subscription_create_response = billing_handler
        .create_subscription_on_connector(
            &state,
            subscription,
            request.item_price_id,
            request.billing,
        )
        .await?;

    let invoice_details = subscription_create_response.invoice_details;
    let invoice_entry = invoice_handler
        .update_invoice(
            &state,
            invoice.id,
            payment_response.payment_method_id.clone(),
            invoice_details
                .clone()
                .and_then(|invoice| invoice.status)
                .unwrap_or(connector_enums::InvoiceStatus::InvoiceCreated),
        )
        .await?;

    subscription_entry
        .update_subscription(diesel_models::subscription::SubscriptionUpdate::new(
            payment_response.payment_method_id.clone(),
            Some(SubscriptionStatus::from(subscription_create_response.status).to_string()),
            Some(
                subscription_create_response
                    .subscription_id
                    .get_string_repr()
                    .to_string(),
            ),
        ))
        .await?;

    let response = subscription_entry.generate_response(
        &invoice_entry,
        &payment_response,
        subscription_create_response.status,
    )?;

    Ok(ApplicationResponse::Json(response))
}

pub async fn get_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: common_utils::id_type::ProfileId,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<SubscriptionResponse> {
    let profile =
        SubscriptionHandler::find_business_profile(&state, &merchant_context, &profile_id)
            .await
            .attach_printable(
                "subscriptions: failed to find business profile in get_subscription",
            )?;
    let handler = SubscriptionHandler::new(&state, &merchant_context, profile);
    let subscription = handler
        .find_subscription(subscription_id)
        .await
        .attach_printable("subscriptions: failed to get subscription entry in get_subscription")?;

    Ok(ApplicationResponse::Json(
        subscription.to_subscription_response(),
    ))
}

pub struct SubscriptionHandler<'a> {
    state: &'a SessionState,
    merchant_context: &'a MerchantContext,
    profile: hyperswitch_domain_models::business_profile::Profile,
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
    handler: &'a SubscriptionHandler<'a>,
    subscription: diesel_models::subscription::Subscription,
    profile: hyperswitch_domain_models::business_profile::Profile,
    merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
}

impl SubscriptionWithHandler<'_> {
    fn generate_response(
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

    async fn update_subscription(
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

pub struct BillingHandler {
    auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType,
    connector_data: api_types::ConnectorData,
    connector_params: hyperswitch_domain_models::connector_endpoints::ConnectorParams,
    connector_metadata: Option<pii::SecretSerdeValue>,
    customer: hyperswitch_domain_models::customer::Customer,
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
}

pub struct InvoiceHandler {
    subscription: diesel_models::subscription::Subscription,
    merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
    profile: hyperswitch_domain_models::business_profile::Profile,
}

#[allow(clippy::todo)]
impl InvoiceHandler {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_invoice_entry(
        self,
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
    pub fn generate_payment_id() -> Option<common_utils::id_type::PaymentId> {
        common_utils::id_type::PaymentId::wrap(common_utils::generate_id_with_default_len(
            "sub_pay",
        ))
        .ok()
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
            confirm: false,
            customer_id: Some(self.subscription.customer_id.clone()),
            payment_id: Self::generate_payment_id(),
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

    pub async fn create_payment(
        &self,
        state: &SessionState,
        request: &subscription_types::CreateSubscriptionRequest,
        amount: MinorUnit,
        currency: common_enums::Currency,
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        let payment_details = &request.payment_details;
        let cit_payment_request = subscription_types::CreatePaymentsRequestData {
            amount,
            currency,
            confirm: true,
            customer_id: Some(self.subscription.customer_id.clone()),
            payment_id: Self::generate_payment_id(),
            billing: request.billing.clone(),
            shipping: request.shipping.clone(),
            setup_future_usage: payment_details.setup_future_usage,
            return_url: Some(payment_details.return_url.clone()),
            capture_method: payment_details.capture_method,
            authentication_type: payment_details.authentication_type,
        };
        payments_api_client::PaymentsApiClient::create_cit_payment(
            state,
            cit_payment_request,
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
            payment_id: Self::generate_payment_id(),
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
            customer_id: Some(self.subscription.customer_id.clone()),
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

    pub async fn create_invoice_record_back_job(
        &self,
        // _invoice: &subscription_types::Invoice,
        _payment_response: &subscription_types::PaymentResponseData,
    ) -> errors::RouterResult<()> {
        // Create an invoice job entry based on payment status
        todo!("Create an invoice job entry based on payment status")
    }
}

#[allow(clippy::todo)]
impl BillingHandler {
    pub async fn create(
        state: &SessionState,
        merchant_context: &MerchantContext,
        customer: hyperswitch_domain_models::customer::Customer,
        profile: hyperswitch_domain_models::business_profile::Profile,
    ) -> errors::RouterResult<Self> {
        let merchant_connector_id = profile.get_billing_processor_id()?;

        let billing_processor_mca = state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &(state).into(),
                merchant_context.get_merchant_account().get_id(),
                &merchant_connector_id,
                merchant_context.get_merchant_key_store(),
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_connector_id.get_string_repr().to_string(),
            })?;

        let connector_name = billing_processor_mca.connector_name.clone();

        let auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType =
            payments_core::helpers::MerchantConnectorAccountType::DbVal(Box::new(
                billing_processor_mca.clone(),
            ))
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "connector_account_details".to_string(),
                expected_format: "auth_type and api_key".to_string(),
            })?;

        let connector_data = api_types::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api_types::GetToken::Connector,
            Some(billing_processor_mca.get_id()),
        )
        .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
        .attach_printable(
            "invalid connector name received in billing merchant connector account",
        )?;

        let connector_enum = connector_enums::Connector::from_str(connector_name.as_str())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("unable to parse connector name {connector_name:?}"))?;

        let connector_params =
            hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
                &state.conf.connectors,
                connector_enum,
            )
            .change_context(errors::ApiErrorResponse::ConfigNotFound)
            .attach_printable(format!(
                "cannot find connector params for this connector {connector_name} in this flow",
            ))?;

        Ok(Self {
            auth_type,
            connector_data,
            connector_params,
            connector_metadata: billing_processor_mca.metadata.clone(),
            customer,
            merchant_connector_id,
        })
    }
    pub async fn create_customer_on_connector(
        &self,
        state: &SessionState,
        customer_id: common_utils::id_type::CustomerId,
        billing_address: Option<api_models::payments::Address>,
        payment_method_data: Option<api_models::payments::PaymentMethodData>,
    ) -> errors::RouterResult<ConnectorCustomerResponseData> {
        let customer_req = ConnectorCustomerData {
            email: self.customer.email.clone().map(pii::Email::from),
            payment_method_data: payment_method_data.clone().map(|pmd| pmd.into()),
            description: None,
            phone: None,
            name: None,
            preprocessing_id: None,
            split_payments: None,
            setup_future_usage: None,
            customer_acceptance: None,
            customer_id: Some(customer_id.clone()),
            billing_address: billing_address
                .as_ref()
                .and_then(|add| add.address.clone())
                .and_then(|addr| addr.into()),
        };
        let router_data = self.build_router_data(
            state,
            customer_req,
            SubscriptionCustomerData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;
        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = Box::pin(self.call_connector(
            state,
            router_data,
            "create customer on connector",
            connector_integration,
        ))
        .await?;

        match response {
            Ok(response_data) => match response_data {
                PaymentsResponseData::ConnectorCustomerResponse(customer_response) => {
                    Ok(customer_response)
                }
                _ => Err(errors::ApiErrorResponse::SubscriptionError {
                    operation: "Subscription Customer Create".to_string(),
                }
                .into()),
            },
            Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: self.connector_data.connector_name.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            }
            .into()),
        }
    }
    pub async fn create_subscription_on_connector(
        &self,
        state: &SessionState,
        subscription: diesel_models::subscription::Subscription,
        item_price_id: Option<String>,
        billing_address: Option<api_models::payments::Address>,
    ) -> errors::RouterResult<subscription_response_types::SubscriptionCreateResponse> {
        let subscription_item = subscription_request_types::SubscriptionItem {
            item_price_id: item_price_id.ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "item_price_id",
            })?,
            quantity: Some(1),
        };
        let subscription_req = subscription_request_types::SubscriptionCreateRequest {
            subscription_id: subscription.id.to_owned(),
            customer_id: subscription.customer_id.to_owned(),
            subscription_items: vec![subscription_item],
            billing_address: billing_address.ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "billing",
                },
            )?,
            auto_collection: subscription_request_types::SubscriptionAutoCollection::Off,
            connector_params: self.connector_params.clone(),
        };

        let router_data = self.build_router_data(
            state,
            subscription_req,
            SubscriptionCreateData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;
        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "create subscription on connector",
                connector_integration,
            )
            .await?;

        match response {
            Ok(response_data) => Ok(response_data),
            Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: self.connector_data.connector_name.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            }
            .into()),
        }
    }

    async fn call_connector<F, ResourceCommonData, Req, Resp>(
        &self,
        state: &SessionState,
        router_data: hyperswitch_domain_models::router_data_v2::RouterDataV2<
            F,
            ResourceCommonData,
            Req,
            Resp,
        >,
        operation_name: &str,
        connector_integration: hyperswitch_interfaces::connector_integration_interface::BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
    ) -> errors::RouterResult<Result<Resp, hyperswitch_domain_models::router_data::ErrorResponse>>
    where
        F: Clone + std::fmt::Debug + 'static,
        Req: Clone + std::fmt::Debug + 'static,
        Resp: Clone + std::fmt::Debug + 'static,
        ResourceCommonData:
            hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    F,
                    Req,
                    Resp,
                > + Clone
                + 'static,
    {
        let old_router_data = ResourceCommonData::to_old_router_data(router_data).change_context(
            errors::ApiErrorResponse::SubscriptionError {
                operation: { operation_name.to_string() },
            },
        )?;

        let router_resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &old_router_data,
            payments_core::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::SubscriptionError {
            operation: operation_name.to_string(),
        })
        .attach_printable(format!(
            "Failed while in subscription operation: {operation_name}"
        ))?;

        Ok(router_resp.response)
    }

    fn build_router_data<F, ResourceCommonData, Req, Resp>(
        &self,
        state: &SessionState,
        req: Req,
        resource_common_data: ResourceCommonData,
    ) -> errors::RouterResult<
        hyperswitch_domain_models::router_data_v2::RouterDataV2<F, ResourceCommonData, Req, Resp>,
    > {
        Ok(hyperswitch_domain_models::router_data_v2::RouterDataV2 {
            flow: std::marker::PhantomData,
            connector_auth_type: self.auth_type.clone(),
            resource_common_data,
            tenant_id: state.tenant.tenant_id.clone(),
            request: req,
            response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
        })
    }
}
