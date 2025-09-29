use std::str::FromStr;

use api_models::{
    enums as api_enums,
    subscription::{self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus},
};
use common_utils::{ext_traits::ValueExt, id_type::GenerateId, pii};
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
use masking::Secret;

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
    let merchant_id = merchant_context.get_merchant_account().get_id();

    let customer = state
        .store
        .find_customer_by_customer_id_merchant_id(
            key_manager_state,
            &request.customer_id,
            merchant_id,
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

    let payment_response = invoice_handler
        .create_cit_payment(&handler.state, &handler.request)
        .await?;

    let invoice_entry = invoice_handler
        .create_invoice_entry(
            &handler.state,
            subscription_entry.profile.get_billing_processor_id()?,
            Some(payment_response.payment_id),
            billing_handler.request.amount,
            billing_handler.request.currency.to_string(),
            common_enums::connector_enums::InvoiceStatus::InvoiceCreated,
            billing_handler.connector_data.connector_name,
            None,
        )
        .await?;

    // invoice_entry
    //     .create_invoice_record_back_job(&payment_response)
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
            merchant_account: self.merchant_context.get_merchant_account().clone(),
        })
    }
}
pub struct SubscriptionWithHandler<'a> {
    handler: &'a SubscriptionHandler,
    subscription: diesel_models::subscription::Subscription,
    profile: hyperswitch_domain_models::business_profile::Profile,
    merchant_account: hyperswitch_domain_models::merchant_account::MerchantAccount,
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
        let mca_id = self.profile.get_billing_processor_id()?;

        let billing_processor_mca = self
            .handler
            .state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &(&self.handler.state).into(),
                self.handler
                    .merchant_context
                    .get_merchant_account()
                    .get_id(),
                &mca_id,
                self.handler.merchant_context.get_merchant_key_store(),
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
            &self.handler.state.conf.connectors,
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
                &self.handler.state.conf.connectors,
                connector_enum,
            )
            .change_context(errors::ApiErrorResponse::ConfigNotFound)
            .attach_printable(format!(
                "cannot find connector params for this connector {connector_name} in this flow",
            ))?;

        Ok(BillingHandler {
            subscription: self.subscription.clone(),
            auth_type,
            connector_data,
            connector_params,
            request: self.handler.request.clone(),
            connector_metadata: billing_processor_mca.metadata.clone(),
            customer,
        })
    }

    pub async fn get_invoice_handler(&self) -> errors::RouterResult<InvoiceHandler> {
        Ok(InvoiceHandler {
            subscription: self.subscription.clone(),
            merchant_account: self.merchant_account.clone(),
            profile: self.profile.clone(),
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
    request: subscription_types::ConfirmSubscriptionRequest,
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

    pub fn generate_payment_id() -> Option<common_utils::id_type::PaymentId> {
        common_utils::id_type::PaymentId::wrap(common_utils::generate_id_with_default_len(
            "subs_pay",
        ))
        .ok()
    }

    pub async fn create_cit_payment(
        &self,
        state: &SessionState,
        request: &subscription_types::ConfirmSubscriptionRequest,
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        let cit_payment_request = subscription_types::PaymentsRequestData {
            amount: Some(request.amount),
            currency: Some(request.currency),
            confirm: true,
            customer_id: Some(self.subscription.customer_id.clone()),
            payment_id: Self::generate_payment_id(),
            payment_details: request.payment_details.clone(),
        };
        payments_api_client::PaymentsApiClient::create_cit_payment(
            state,
            cit_payment_request,
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
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
    pub async fn create_customer_on_connector(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<ConnectorCustomerResponseData> {
        let customer_req = ConnectorCustomerData {
            email: self.customer.email.clone().map(pii::Email::from),
            payment_method_data: self
                .request
                .payment_details
                .payment_method_data
                .payment_method_data
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
                .request
                .billing_address
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
    ) -> errors::RouterResult<subscription_response_types::SubscriptionCreateResponse> {
        let subscription_item = subscription_request_types::SubscriptionItem {
            item_price_id: self.request.get_item_price_id().change_context(
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
            billing_address: self.request.get_billing_address().change_context(
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
