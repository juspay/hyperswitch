use std::str::FromStr;

use api_models::{
    enums as api_enums, process_tracker as pt_types,
    subscription::{self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus},
};
use common_utils::{ext_traits::ValueExt, id_type::GenerateId, pii};
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    merchant_context::MerchantContext,
    router_data_v2::flow_common_types::{
        InvoiceRecordBackData, SubscriptionCreateData, SubscriptionCustomerData,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest, subscriptions as subscription_request_types,
        ConnectorCustomerData,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse, subscriptions as subscription_response_types,
        ConnectorCustomerResponseData, PaymentsResponseData,
    },
};
use masking::Secret;

use super::errors::{self, RouterResponse};
use crate::{
    core::payments as payments_core, routes::SessionState, services, types::api as api_types,
    workflows,
};

pub const SUBSCRIPTION_CONNECTOR_ID: &str = "DefaultSubscriptionConnectorId";
pub const SUBSCRIPTION_PAYMENT_ID: &str = "DefaultSubscriptionPaymentId";

pub async fn create_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: String,
    request: subscription_types::CreateSubscriptionRequest,
) -> RouterResponse<CreateSubscriptionResponse> {
    let store = state.store.clone();
    let db = store.as_ref();
    let id = common_utils::id_type::SubscriptionId::generate();
    let profile_id = common_utils::id_type::ProfileId::from_str(&profile_id).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "X-Profile-Id",
        },
    )?;

    let mut subscription = SubscriptionNew::new(
        id,
        SubscriptionStatus::Created.to_string(),
        None,
        None,
        None,
        None,
        None,
        merchant_context.get_merchant_account().get_id().clone(),
        request.customer_id.clone(),
        None,
        profile_id,
        request.merchant_reference_id,
    );

    subscription.generate_and_set_client_secret();
    let subscription_response = db
        .insert_subscription_entry(subscription)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("subscriptions: unable to insert subscription entry to database")?;

    let response = CreateSubscriptionResponse::new(
        subscription_response.id.clone(),
        subscription_response.merchant_reference_id,
        SubscriptionStatus::from_str(&subscription_response.status)
            .unwrap_or(SubscriptionStatus::Created),
        None,
        subscription_response.profile_id,
        subscription_response.merchant_id,
        subscription_response.client_secret.map(Secret::new),
        request.customer_id,
    );

    Ok(ApplicationResponse::Json(response))
}

pub async fn confirm_subscription(
    state: SessionState,
    merchant_context: MerchantContext,
    profile_id: String,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let profile_id = common_utils::id_type::ProfileId::from_str(&profile_id).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "X-Profile-Id",
        },
    )?;

    let key_manager_state = &(&state).into();
    let merchant_key_store = merchant_context.get_merchant_key_store();

    let profile = state
        .store
        .find_business_profile_by_profile_id(key_manager_state, merchant_key_store, &profile_id)
        .await
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_string(),
        })?;

    let customer = state
        .store
        .find_customer_by_customer_id_merchant_id(
            key_manager_state,
            &request.customer_id,
            merchant_context.get_merchant_account().get_id(),
            merchant_key_store,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("subscriptions: unable to fetch customer from database")?;

    let handler = SubscriptionHandler::new(state, merchant_context, request, profile);

    let mut subscription_entry = handler
        .find_subscription(subscription_id.get_string_repr().to_string())
        .await?;

    let billing_handler = subscription_entry.get_billing_handler(customer).await?;
    let invoice_handler = subscription_entry.get_invoice_handler().await?;

    let _customer_create_response = billing_handler
        .create_customer_on_connector(&handler.state)
        .await?;

    let subscription_create_response = billing_handler
        .create_subscription_on_connector(&handler.state)
        .await?;

    // let payment_response = invoice_handler.create_cit_payment().await?;

    let invoice_entry = invoice_handler
        .create_invoice_entry(
            &handler.state,
            subscription_entry.profile.get_billing_processor_id()?,
            None,
            billing_handler.amount,
            billing_handler.currency.clone().to_string(),
            common_enums::connector_enums::InvoiceStatus::InvoiceCreated,
            billing_handler.connector_data.connector_name,
            None,
        )
        .await?;

    // invoice_handler
    //     .create_invoice_sync_job(
    //         &handler.state,
    //         payment_response,
    //         &invoice_entry,
    //         subscription_create_response.connector_invoice_id.clone(),
    //     )
    //     .await?;

    subscription_entry
        .update_subscription_status(
            SubscriptionStatus::from(subscription_create_response.status).to_string(),
        )
        .await?;

    let response = subscription_entry
        .generate_response(&invoice_entry, subscription_create_response.status)?;

    Ok(ApplicationResponse::Json(response))
}

pub struct SubscriptionHandler {
    state: SessionState,
    merchant_context: MerchantContext,
    request: subscription_types::ConfirmSubscriptionRequest,
    profile: hyperswitch_domain_models::business_profile::Profile,
}

impl SubscriptionHandler {
    pub fn new(
        state: SessionState,
        merchant_context: MerchantContext,
        request: subscription_types::ConfirmSubscriptionRequest,
        profile: hyperswitch_domain_models::business_profile::Profile,
    ) -> Self {
        Self {
            state,
            merchant_context,
            request,
            profile,
        }
    }
    pub async fn find_subscription(
        &self,
        subscription_id: String,
    ) -> errors::RouterResult<SubscriptionWithHandler<'_>> {
        let subscription = self
            .state
            .store
            .find_by_merchant_id_subscription_id(
                self.merchant_context.get_merchant_account().get_id(),
                subscription_id.clone(),
            )
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!("subscription not found for id: {subscription_id}"),
            })?;

        Ok(SubscriptionWithHandler {
            handler: self,
            subscription,
            profile: self.profile.clone(),
        })
    }
}
pub struct SubscriptionWithHandler<'a> {
    handler: &'a SubscriptionHandler,
    subscription: diesel_models::subscription::Subscription,
    profile: hyperswitch_domain_models::business_profile::Profile,
}

impl<'a> SubscriptionWithHandler<'a> {
    fn generate_response(
        &self,
        invoice: &diesel_models::invoice::Invoice,
        // _payment_response: &subscription_types::PaymentResponseData,
        status: subscription_response_types::SubscriptionStatus,
    ) -> errors::RouterResult<subscription_types::ConfirmSubscriptionResponse> {
        Ok(subscription_types::ConfirmSubscriptionResponse {
            id: self.subscription.id.clone(),
            merchant_reference_id: self.subscription.merchant_reference_id.clone(),
            status: SubscriptionStatus::from(status),
            plan_id: None,
            profile_id: self.subscription.profile_id.to_owned(),
            payment: None,
            customer_id: Some(self.subscription.customer_id.clone()),
            price_id: None,
            coupon: None,
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

    async fn update_subscription_status(&mut self, status: String) -> errors::RouterResult<()> {
        let db = self.handler.state.store.as_ref();
        let updated_subscription = db
            .update_subscription_entry(
                self.handler
                    .merchant_context
                    .get_merchant_account()
                    .get_id(),
                self.subscription.id.get_string_repr().to_string(),
                diesel_models::subscription::SubscriptionUpdate::new(None, Some(status)),
            )
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Subscription Update".to_string(),
            })
            .attach_printable("subscriptions: unable to update subscription entry in database")?;

        self.subscription = updated_subscription;

        Ok(())
    }

    pub async fn get_billing_handler(
        &self,
        customer: hyperswitch_domain_models::customer::Customer,
    ) -> errors::RouterResult<BillingHandler> {
        let handler = BillingHandler::create(
            &self.handler.state,
            self.handler.merchant_context.get_merchant_account(),
            self.handler.merchant_context.get_merchant_key_store(),
            self.subscription.clone(),
            customer,
            self.profile.clone(),
            self.handler.request.item_price_id.clone(),
            self.handler.request.billing_address.clone(),
            Some(self.handler.request.payment_details.clone()),
            self.handler.request.amount,
            self.handler.request.currency,
        )
        .await?;

        Ok(handler)
    }

    pub async fn get_invoice_handler(&self) -> errors::RouterResult<InvoiceHandler> {
        Ok(InvoiceHandler {
            subscription: self.subscription.clone(),
        })
    }
}

pub struct BillingHandler {
    subscription: diesel_models::subscription::Subscription,
    auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType,
    connector_data: api_types::ConnectorData,
    connector_params: hyperswitch_domain_models::connector_endpoints::ConnectorParams,
    connector_metadata: Option<pii::SecretSerdeValue>,
    customer: hyperswitch_domain_models::customer::Customer,
    item_price_id: Option<String>,
    billing_address: Option<api_models::payments::Address>,
    payment_details: Option<subscription_types::PaymentDetails>,
    amount: common_utils::types::MinorUnit,
    currency: api_enums::Currency,
}

pub struct InvoiceHandler {
    subscription: diesel_models::subscription::Subscription,
}

#[allow(clippy::todo)]
impl InvoiceHandler {
    pub fn new(subscription: diesel_models::subscription::Subscription) -> Self {
        Self { subscription }
    }
    pub async fn fetch_invoice_by_id(
        &self,
        state: &SessionState,
        id: &common_utils::id_type::InvoiceId,
    ) -> errors::RouterResult<diesel_models::invoice::Invoice> {
        // Fetch invoice from DB
        state
            .store
            .find_invoice_by_invoice_id(id.get_string_repr().to_string())
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!("invoice not found for id: {}", id.get_string_repr()),
            })
            .attach_printable("invoices: unable to fetch invoice from database")
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_invoice_entry(
        &self,
        state: &SessionState,
        merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
        payment_intent_id: Option<common_utils::id_type::PaymentId>,
        amount: common_utils::types::MinorUnit,
        currency: String,
        status: common_enums::connector_enums::InvoiceStatus,
        provider_name: common_enums::connector_enums::Connector,
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
            currency,
            status,
            provider_name,
            metadata,
        );

        let invoice = state
            .store
            .insert_invoice_entry(invoice_new)
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Subscription Confirm".to_string(),
            })
            .attach_printable("invoices: unable to insert invoice entry to database")?;

        Ok(invoice)
    }

    pub async fn update_invoice_status(
        &self,
        state: &SessionState,
        invoice_id: String,
        status: common_enums::connector_enums::InvoiceStatus,
    ) -> errors::RouterResult<diesel_models::invoice::Invoice> {
        let update = diesel_models::invoice::InvoiceUpdate::new(None, Some(status));

        let updated_invoice = state
            .store
            .update_invoice_entry(invoice_id, update)
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Invoice Update".to_string(),
            })
            .attach_printable("invoices: unable to update invoice entry in database")?;

        Ok(updated_invoice)
    }

    pub async fn create_cit_payment(
        &self,
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        // Create a CIT payment for the invoice
        todo!("Create a CIT payment for the invoice")
    }

    pub async fn create_invoice_sync_job(
        &self,
        state: &SessionState,
        payment_response: &subscription_types::PaymentResponseData,
        invoice: &diesel_models::invoice::Invoice,
        connector_invoice_id: String,
    ) -> errors::RouterResult<()> {
        // Create an invoice job entry based on payment status

        let invoice_sync_request = pt_types::invoice_sync::InvoiceSyncRequest::new(
            payment_response.payment_id.to_owned(),
            self.subscription.id.to_owned(),
            invoice.merchant_connector_id.to_owned(),
            invoice.id.to_owned(),
            invoice.merchant_id.to_owned(),
            invoice.profile_id.to_owned(),
            invoice.customer_id.to_owned(),
            payment_response.amount,
            payment_response.currency,
            None,
            payment_response.status,
            connector_invoice_id,
        );

        workflows::invoice_sync::create_invoice_sync_job(state, invoice_sync_request)
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Create Invoice Record back job".to_string(),
            })
            .attach_printable("invoices: unable to update invoice entry in database")
    }
}

#[allow(clippy::todo)]
impl BillingHandler {
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        state: &SessionState,
        merchant_account: &hyperswitch_domain_models::merchant_account::MerchantAccount,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        subscription: diesel_models::subscription::Subscription,
        customer: hyperswitch_domain_models::customer::Customer,
        profile: hyperswitch_domain_models::business_profile::Profile,
        item_price_id: Option<String>,
        billing_address: Option<api_models::payments::Address>,
        payment_details: Option<subscription_types::PaymentDetails>,
        amount: common_utils::types::MinorUnit,
        currency: api_enums::Currency,
    ) -> errors::RouterResult<Self> {
        let mca_id = profile.get_billing_processor_id()?;

        let billing_processor_mca = state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &(state).into(),
                merchant_account.get_id(),
                &mca_id,
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: mca_id.get_string_repr().to_string(),
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

        let connector_enum =
            common_enums::connector_enums::Connector::from_str(connector_name.as_str())
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
            subscription,
            auth_type,
            connector_data,
            connector_params,
            connector_metadata: billing_processor_mca.metadata.clone(),
            customer,
            item_price_id,
            billing_address,
            payment_details,
            amount,
            currency,
        })
    }
    pub async fn create_customer_on_connector(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<ConnectorCustomerResponseData> {
        let customer_req = ConnectorCustomerData {
            email: self.customer.email.clone().map(pii::Email::from),
            payment_method_data: self
                .payment_details
                .as_ref()
                .and_then(|details| details.payment_method_data.payment_method_data.clone())
                .clone()
                .map(|pmd| pmd.into()),
            description: None,
            phone: None,
            name: None,
            preprocessing_id: None,
            split_payments: None,
            setup_future_usage: None,
            customer_acceptance: None,
            customer_id: Some(self.subscription.customer_id.to_owned()),
            billing_address: self
                .billing_address
                .as_ref()
                .and_then(|add| add.address.clone()),
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
    ) -> errors::RouterResult<subscription_response_types::SubscriptionCreateResponse> {
        let subscription_item = subscription_request_types::SubscriptionItem {
            item_price_id: self.item_price_id.clone().ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "item_price_id",
                },
            )?,
            quantity: Some(1),
        };
        let subscription_req = subscription_request_types::SubscriptionCreateRequest {
            subscription_id: self.subscription.id.to_owned(),
            customer_id: self.subscription.customer_id.to_owned(),
            subscription_items: vec![subscription_item],
            billing_address: self.billing_address.clone().ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "billing_address",
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

    pub async fn record_back_to_billing_processor(
        &self,
        state: &SessionState,
        invoice_id: String,
    ) -> errors::RouterResult<InvoiceRecordBackResponse> {
        let invoice_record_back_req = InvoiceRecordBackRequest {
            amount: self.amount,
            currency: self.currency,
            payment_method_type: self
                .payment_details
                .as_ref()
                .and_then(|details| details.payment_method_type),
            attempt_status: common_enums::AttemptStatus::Charged,
            merchant_reference_id: common_utils::id_type::PaymentReferenceId::from_str(&invoice_id)
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "invoice_id",
                })?,
            connector_params: self.connector_params.clone(),
            connector_transaction_id: None,
        };

        let router_data = self.build_router_data(
            state,
            invoice_record_back_req,
            InvoiceRecordBackData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;
        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "invoice record back",
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
