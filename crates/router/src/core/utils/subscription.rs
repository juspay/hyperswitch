use std::str::FromStr;

use api_models::{enums as api_enums, subscription as subscription_types};
use common_utils::{
    ext_traits::{OptionExt, ValueExt},
    pii,
};
use diesel_models::subscription::{self, Subscription};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::flow_common_types::{
        GetSubscriptionPlanPricesData, GetSubscriptionPlansData, SubscriptionCreateData,
        SubscriptionCustomerData,
    },
    router_flow_types::subscriptions as subscription_flow,
    router_request_types::{subscriptions as subscription_request, ConnectorCustomerData},
    router_response_types::{
        subscriptions as subscription_response, ConnectorCustomerResponseData, PaymentsResponseData,
    },
    subscription::ClientSecret,
};

use crate::{
    consts, core::payments as payments_core, db::errors, routes::SessionState, services,
    types::api as api_types,
};

pub const SUBSCRIPTION_CONNECTOR_ID: &str = "DefaultSubscriptionConnectorId";
pub const SUBSCRIPTION_PAYMENT_ID: &str = "DefaultSubscriptionPaymentId";

pub struct SubscriptionHandler {
    pub state: SessionState,
    pub merchant_context: MerchantContext,
    pub profile: hyperswitch_domain_models::business_profile::Profile,
}

pub struct SubscriptionWithHandler<'a> {
    pub handler: &'a SubscriptionHandler,
    pub subscription: Option<diesel_models::subscription::Subscription>,
    pub profile: hyperswitch_domain_models::business_profile::Profile,
}

pub struct InvoiceHandler {
    pub subscription: diesel_models::subscription::Subscription,
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

    pub async fn create_cit_payment(
        &self,
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        // Create a CIT payment for the invoice
        todo!("Create a CIT payment for the invoice")
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

impl<'a> SubscriptionWithHandler<'a> {
    pub fn new(handler: &'a SubscriptionHandler, subscription: Option<Subscription>) -> Self {
        Self {
            handler,
            subscription,
            profile: handler.profile.clone(),
        }
    }

    pub fn generate_response(
        &self,
        invoice: &diesel_models::invoice::Invoice,
        // _payment_response: &subscription_types::PaymentResponseData,
        status: subscription_response::SubscriptionStatus,
    ) -> errors::RouterResult<subscription_types::ConfirmSubscriptionResponse> {
        let subscription =
            self.subscription
                .clone()
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "subscription",
                })?;

        Ok(subscription_types::ConfirmSubscriptionResponse {
            id: subscription.id.clone(),
            merchant_reference_id: subscription.merchant_reference_id.clone(),
            status: subscription_types::SubscriptionStatus::from(status),
            plan_id: None,
            profile_id: subscription.profile_id.to_owned(),
            payment: None,
            customer_id: Some(subscription.customer_id.clone()),
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

    pub async fn update_subscription_status(&mut self, status: String) -> errors::RouterResult<()> {
        let subscription =
            self.subscription
                .clone()
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "subscription",
                })?;

        let db = self.handler.state.store.as_ref();
        let updated_subscription = db
            .update_subscription_entry(
                self.handler
                    .merchant_context
                    .get_merchant_account()
                    .get_id(),
                subscription.id.get_string_repr().to_string(),
                diesel_models::subscription::SubscriptionUpdate::new(None, Some(status)),
            )
            .await
            .change_context(errors::ApiErrorResponse::SubscriptionError {
                operation: "Subscription Update".to_string(),
            })
            .attach_printable("subscriptions: unable to update subscription entry in database")?;

        self.subscription = Some(updated_subscription);

        Ok(())
    }

    pub async fn get_billing_handler(
        &self,
        customer: Option<hyperswitch_domain_models::customer::Customer>,
        confirm_request: Option<&subscription_types::ConfirmSubscriptionRequest>,
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
            connector_metadata: billing_processor_mca.metadata.clone(),
            customer,
            billing_address: confirm_request.and_then(|cr| cr.billing_address.clone()),
            payment_details: confirm_request.map(|cr| cr.payment_details.clone()),
            item_price_id: confirm_request.and_then(|cr| cr.item_price_id.clone()),
        })
    }

    pub async fn get_invoice_handler(&self) -> errors::RouterResult<InvoiceHandler> {
        let subscription =
            self.subscription
                .clone()
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "subscription",
                })?;

        Ok(InvoiceHandler { subscription })
    }
}

impl SubscriptionHandler {
    pub fn new(
        state: SessionState,
        merchant_context: MerchantContext,
        profile: hyperswitch_domain_models::business_profile::Profile,
    ) -> Self {
        Self {
            state,
            merchant_context,
            profile,
        }
    }

    pub async fn find_and_validate_subscription(
        &self,
        client_secret: &ClientSecret,
    ) -> errors::RouterResult<()> {
        let subscription_id = client_secret.get_subscription_id()?;

        let subscription = self
            .state
            .store
            .find_by_merchant_id_subscription_id(
                self.merchant_context.get_merchant_account().get_id(),
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
            subscription: Some(subscription),
            profile: self.profile.clone(),
        })
    }

    /// Tail helper for secret + expiry validation
    pub fn validate_client_secret(
        &self,
        client_secret: &ClientSecret,
        subscription: &Subscription,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
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
}

pub struct BillingHandler {
    pub subscription: Option<Subscription>,
    pub auth_type: ConnectorAuthType,
    pub connector_data: crate::types::api::ConnectorData,
    pub connector_params: hyperswitch_domain_models::connector_endpoints::ConnectorParams,
    pub connector_metadata: Option<pii::SecretSerdeValue>,
    pub customer: Option<hyperswitch_domain_models::customer::Customer>,
    pub billing_address: Option<api_models::payments::Address>,
    pub payment_details: Option<subscription_types::PaymentDetails>,
    pub item_price_id: Option<String>,
}

#[allow(clippy::todo)]
impl BillingHandler {
    pub async fn get_subscription_plans(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<subscription_response::GetSubscriptionPlansResponse> {
        let get_plans_request = subscription_request::GetSubscriptionPlansRequest::default();

        let router_data = self.build_router_data(
            state,
            get_plans_request,
            GetSubscriptionPlansData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;

        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "get subscription plans",
                connector_integration,
            )
            .await?;

        match response {
            Ok(resp) => Ok(resp),
            Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: self.connector_data.connector_name.to_string().clone(),
                status_code: err.status_code,
                reason: err.reason,
            }
            .into()),
        }
    }

    pub async fn get_subscription_plan_prices(
        &self,
        state: &SessionState,
        plan_price_id: String,
    ) -> errors::RouterResult<subscription_response::GetSubscriptionPlanPricesResponse> {
        let get_plan_prices_request =
            subscription_request::GetSubscriptionPlanPricesRequest { plan_price_id };

        let router_data = self.build_router_data(
            state,
            get_plan_prices_request,
            GetSubscriptionPlanPricesData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;

        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "get subscription plan prices",
                connector_integration,
            )
            .await?;

        match response {
            Ok(resp) => Ok(resp),
            Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: self.connector_data.connector_name.to_string().clone(),
                status_code: err.status_code,
                reason: err.reason,
            }
            .into()),
        }
    }

    pub async fn create_customer_on_connector(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<ConnectorCustomerResponseData> {
        let customer =
            self.customer
                .clone()
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "customer",
                })?;

        let customer_req = ConnectorCustomerData {
            email: customer.email.clone().map(pii::Email::from),
            payment_method_data: self
                .payment_details
                .as_ref()
                .map(|pd| pd.payment_method_data.clone())
                .and_then(|pmd| pmd.payment_method_data)
                .clone()
                .map(|pmd| pmd.into()),
            description: None,
            phone: None,
            name: None,
            preprocessing_id: None,
            split_payments: None,
            setup_future_usage: None,
            customer_acceptance: None,
            customer_id: Some(customer.get_id().to_owned()),
            billing_address: self
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
    ) -> errors::RouterResult<subscription_response::SubscriptionCreateResponse> {
        let subscription =
            self.subscription
                .clone()
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "subscription",
                })?;

        let subscription_item = subscription_request::SubscriptionItem {
            item_price_id: self.item_price_id.clone().ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "item_price_id",
                },
            )?,
            quantity: Some(1),
        };
        let subscription_req = subscription_request::SubscriptionCreateRequest {
            subscription_id: subscription.id.to_owned(),
            customer_id: subscription.customer_id.to_owned(),
            subscription_items: vec![subscription_item],
            billing_address: self.billing_address.clone().ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "billing_address",
                },
            )?,
            auto_collection: subscription_request::SubscriptionAutoCollection::Off,
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
