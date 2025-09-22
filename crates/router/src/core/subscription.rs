use std::str::FromStr;

use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus,
};
use common_utils::{ext_traits::ValueExt, id_type::GenerateId, pii};
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{api::ApplicationResponse, merchant_context::MerchantContext};
use masking::Secret;

use super::errors::{self, RouterResponse};
use crate::{
    core::payments as payments_core, routes::SessionState, services, types::api as api_types,
};

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
    _profile_id: String,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: common_utils::id_type::SubscriptionId,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let handler = SubscriptionHandler::new(state, merchant_context, request);

    let mut subscription_entry = handler
        .find_subscription(subscription_id.get_string_repr().to_string())
        .await?;

    let billing_handler = subscription_entry.get_billing_handler().await?;
    let _invoice_handler = subscription_entry.get_invoice_handler().await?;

    let _customer_create_response = billing_handler.create_customer(&handler.state).await?;

    let subscription_create_response = billing_handler.create_subscription(&handler.state).await?;

    // let invoice = invoice_handler.create_invoice_in_db().await?;
    // let payment_response = invoice_handler.create_cit_payment().await?;

    // invoice_handler
    //     .create_invoice_job(&payment_response)
    //     .await?;

    // invoice_handler.update_invoice_record().await?;
    subscription_entry
        .update_subscription_status(
            SubscriptionStatus::from(subscription_create_response.status).to_string(),
        )
        .await?;

    let response = subscription_entry.generate_response(subscription_create_response.status)?;

    Ok(ApplicationResponse::Json(response))
}

#[allow(dead_code)]
pub struct SubscriptionHandler {
    state: SessionState,
    merchant_context: MerchantContext,
    request: subscription_types::ConfirmSubscriptionRequest,
}

impl SubscriptionHandler {
    pub fn new(
        state: SessionState,
        merchant_context: MerchantContext,
        request: subscription_types::ConfirmSubscriptionRequest,
    ) -> Self {
        Self {
            state,
            merchant_context,
            request,
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
        })
    }
}
pub struct SubscriptionWithHandler<'a> {
    handler: &'a SubscriptionHandler,
    subscription: diesel_models::subscription::Subscription,
}

#[allow(clippy::todo)]
impl<'a> SubscriptionWithHandler<'a> {
    fn generate_response(
        &self,
        // _invoice: &subscription_types::Invoice,
        // _payment_response: &subscription_types::PaymentResponseData,
        status: hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionStatus,
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
            // invoice: Some(invoice.clone()),
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
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("subscriptions: unable to update subscription entry in database")?;

        self.subscription = updated_subscription;

        Ok(())
    }
    async fn get_invoice_handler(&self) -> errors::RouterResult<InvoiceHandler> {
        Ok(InvoiceHandler {
            subscription: self.subscription.clone(),
        })
    }
    pub async fn get_billing_handler(&self) -> errors::RouterResult<BillingHandler> {
        // let mca_id = self.subscription.merchant_connector_id.clone().ok_or(
        //     errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
        //         id: "No mca_id associated with this subscription".to_string(),
        //     },
        // )?;

        let mca_id = common_utils::id_type::MerchantConnectorAccountId::wrap(
            "mca_aR9xLgJB3K1CWabu8g2X".to_string(),
        )
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "merchant_connector_account_id",
        })?;

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
            connector_name,
            auth_type,
            connector_data,
            connector_params,
            request: self.handler.request.clone(),
            connector_metadata: billing_processor_mca.metadata.clone(),
        })
    }
}

#[allow(dead_code)]
pub struct BillingHandler {
    subscription: diesel_models::subscription::Subscription,
    connector_name: String,
    auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType,
    connector_data: api_types::ConnectorData,
    connector_params: hyperswitch_domain_models::connector_endpoints::ConnectorParams,
    connector_metadata: Option<pii::SecretSerdeValue>,
    request: subscription_types::ConfirmSubscriptionRequest,
}

#[allow(dead_code)]
pub struct InvoiceHandler {
    subscription: diesel_models::subscription::Subscription,
    // An invoice diesel type to be added here
}

#[allow(dead_code)]
pub struct SubscriptionCreatedBilling<'a> {
    billing_handler: &'a BillingHandler,
    subscription_response: CreateSubscriptionResponse,
}

#[allow(clippy::todo)]
impl InvoiceHandler {
    pub async fn create_invoice_in_db(&self) -> errors::RouterResult<()> {
        // Create invoice in DB and return the invoice details
        todo!("Create invoice in DB and return the invoice details")
    }
    pub async fn create_invoice_job(
        &self,
        // _invoice: &subscription_types::Invoice,
        _payment_response: &subscription_types::PaymentResponseData,
    ) -> errors::RouterResult<()> {
        // Create an invoice job entry based on payment status
        todo!("Create an invoice job entry based on payment status")
    }

    pub async fn create_cit_payment(
        &self,
    ) -> errors::RouterResult<subscription_types::PaymentResponseData> {
        // Create a CIT payment for the invoice
        todo!("Create a CIT payment for the invoice")
    }

    pub async fn update_invoice_record(&self) -> errors::RouterResult<()> {
        // Update the invoice record based on payment status
        todo!("Update the invoice record based on payment status")
    }
}

#[allow(clippy::todo)]
impl BillingHandler {
    pub async fn create_customer(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<
        hyperswitch_domain_models::router_response_types::ConnectorCustomerResponseData,
    > {
        let router_data = self.build_customer_router_data(state)?;
        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = self
            .call_connector(state, router_data, "create customer", connector_integration)
            .await?;

        match response {
            Ok(response_data) => match response_data {
                hyperswitch_domain_models::router_response_types::PaymentsResponseData::ConnectorCustomerResponse(customer_response) => {
                    Ok(customer_response)
                }
                _ => Err(errors::ApiErrorResponse::InternalServerError.into()),
            },
            Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: self.connector_name.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            }
            .into()),
        }
    }
    pub async fn create_subscription(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<
        hyperswitch_domain_models::router_response_types::subscriptions::SubscriptionCreateResponse,
    > {
        let router_data = self.build_subscription_router_data(state)?;
        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "create subscription",
                connector_integration,
            )
            .await?;

        match response {
            Ok(response_data) => Ok(response_data),
            Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: self.connector_name.to_string(),
                status_code: err.status_code,
                reason: err.reason,
            }
            .into()),
        }
    }

    async fn call_connector<F, ResourceCommonData, Req, Resp>(
        &self,
        state: &SessionState,
        router_data: hyperswitch_domain_models::router_data::RouterData<F, Req, Resp>,
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
        // Uncomment the below code once the connector integration is done
        let router_resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments_core::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed while calling {operation_name} at billing processor"
        ))?;

        Ok(router_resp.response)

        // todo!("Call the connector and return the response")
    }
    fn build_customer_router_data(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<hyperswitch_domain_models::types::ConnectorCustomerRouterData> {
        // Build customer creation router data

        let customer_req = hyperswitch_domain_models::router_request_types::ConnectorCustomerData {
            email: self.request.customer.as_ref().and_then(|c| c.email.clone()),
            payment_method_data: self
                .request
                .payment_data
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

        Ok(hyperswitch_domain_models::router_data::RouterData {
            flow: std::marker::PhantomData,
            merchant_id: self.subscription.merchant_id.to_owned(),
            customer_id: Some(self.subscription.customer_id.to_owned()),
            connector_customer: None,
            connector: self.connector_name.clone(),
            payment_id: "DefaultPaymentId".to_string(),
            tenant_id: state.tenant.tenant_id.clone(),
            attempt_id: "Subscriptions attempt".to_owned(),
            status: common_enums::AttemptStatus::default(),
            payment_method: common_enums::PaymentMethod::default(),
            connector_auth_type: self.auth_type.clone(),
            description: None,
            address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
            auth_type: common_enums::AuthenticationType::default(),
            connector_meta_data: self.connector_metadata.clone(),
            connector_wallets_details: None,
            amount_captured: None,
            minor_amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            payment_method_balance: None,
            connector_api_version: None,
            request: customer_req,
            response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
            connector_request_reference_id: "Notjing".to_owned(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            dispute_id: None,
            refund_id: None,
            payment_method_status: None,
            connector_response: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload: None,
            connector_mandate_request_reference_id: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            l2_l3_data: None,
            minor_amount_capturable: None,
        })
    }

    fn build_subscription_router_data(
        &self,
        state: &SessionState,
    ) -> errors::RouterResult<hyperswitch_domain_models::types::SubscriptionCreateRouterData> {
        // Build subscription creation router data using customer_id
        let subscription_item =
            hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionItem {
                item_price_id: self.request.item_price_id.clone().ok_or(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: "item_price_id is required".to_string(),
                    },
                )?,
                quantity: Some(1),
            };
        let subscription_req = hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionCreateRequest {
            subscription_id: self.subscription.id.to_owned(),
            customer_id: self.subscription.customer_id.to_owned(),
            subscription_items: vec![subscription_item], // to be filled
            billing_address: self.request.billing_address.clone().ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "billing_address",
            })?,
            auto_collection: hyperswitch_domain_models::router_request_types::subscriptions::SubscriptionAutoCollection::Off,
            connector_params: self.connector_params.clone(),
         };

        Ok(hyperswitch_domain_models::router_data::RouterData {
            flow: std::marker::PhantomData,
            merchant_id: self.subscription.merchant_id.to_owned(),
            customer_id: Some(self.subscription.customer_id.to_owned()),
            connector_customer: None,
            connector: self.connector_name.clone(),
            payment_id: "DefaultPaymentId".to_string(),
            tenant_id: state.tenant.tenant_id.clone(),
            attempt_id: "Subscriptions attempt".to_owned(),
            status: common_enums::AttemptStatus::default(),
            payment_method: common_enums::PaymentMethod::default(),
            connector_auth_type: self.auth_type.clone(),
            description: None,
            address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
            auth_type: common_enums::AuthenticationType::default(),
            connector_meta_data: self.connector_metadata.clone(),
            connector_wallets_details: None,
            amount_captured: None,
            minor_amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            payment_method_balance: None,
            connector_api_version: None,
            request: subscription_req,
            response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
            connector_request_reference_id: "Notjing".to_owned(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            dispute_id: None,
            refund_id: None,
            payment_method_status: None,
            connector_response: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload: None,
            connector_mandate_request_reference_id: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            l2_l3_data: None,
            minor_amount_capturable: None,
        })
    }
}
