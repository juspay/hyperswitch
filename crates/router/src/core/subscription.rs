use std::str::FromStr;

use api_models::subscription::{
    self as subscription_types, CreateSubscriptionResponse, SubscriptionStatus,
};
use common_utils::ext_traits::ValueExt;
use common_utils::id_type::GenerateId;
use diesel_models::subscription::SubscriptionNew;
use error_stack::ResultExt;
use hyperswitch_domain_models::{api::ApplicationResponse, merchant_context::MerchantContext};
use masking::Secret;

use super::errors::{self, RouterResponse};
use crate::{core::payments as payments_core, routes::SessionState, types::api as api_types};

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
    _authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: subscription_types::ConfirmSubscriptionRequest,
    subscription_id: String,
) -> RouterResponse<subscription_types::ConfirmSubscriptionResponse> {
    let handler = SubscriptionHandler::new(state, merchant_context, request);

    let subscription_entry = handler.find_subscription(subscription_id).await?;

    let billing_handler = subscription_entry.get_billing_handler().await?;
    let invoice_handler = subscription_entry.get_invoice_handler().await?;

    let customer_create_response = billing_handler.create_customer().await?;

    let _subscription_create_response = billing_handler
        .create_subscription(&customer_create_response.id)
        .await?;

    let invoice = invoice_handler.create_invoice_in_db().await?;
    let payment_response = invoice_handler.create_cit_payment().await?;

    invoice_handler
        .create_invoice_job(&payment_response)
        .await?;

    invoice_handler.update_invoice_record().await?;

    let response = subscription_entry.generate_response(&payment_response)?;

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
        _payment_response: &subscription_types::PaymentResponseData,
    ) -> errors::RouterResult<subscription_types::ConfirmSubscriptionResponse> {
        todo!(
            "Generate ConfirmSubscriptionResponse from subscription, invoice and payment_response"
        )
    }
    async fn get_invoice_handler(&self) -> errors::RouterResult<InvoiceHandler> {
        Ok(InvoiceHandler {
            subscription: self.subscription.clone(),
        })
    }
    pub async fn get_billing_handler(&self) -> errors::RouterResult<BillingHandler> {
        let mca_id = self.subscription.merchant_connector_id.clone().ok_or(
            errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: "No mca_id associated with this subscription".to_string(),
            },
        )?;

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
    ) -> errors::RouterResult<subscription_types::CreateCustomerResponse> {
        let router_data = self.build_customer_router_data()?;
        let response = self.call_connector(router_data, "create customer").await?;

        Ok(response)
    }
    pub async fn create_subscription(
        &self,
        customer_id: &common_utils::id_type::CustomerId,
    ) -> errors::RouterResult<subscription_types::SubscriptionCreateResponse> {
        let router_data = self.build_subscription_router_data(customer_id)?;
        let response = self
            .call_connector(router_data, "create subscription")
            .await?;

        Ok(response)
    }

    async fn call_connector<F, Req, Resp>(
        &self,
        _router_data: hyperswitch_domain_models::router_data::RouterData<F, Req, Resp>,
        _operation_name: &str,
    ) -> errors::RouterResult<Resp>
    where
        F: Clone + std::fmt::Debug,
        Req: Clone + std::fmt::Debug,
        Resp: Clone + std::fmt::Debug,
    {
        // Uncomment the below code once the connector integration is done

        // let connector_integration = self.connector_data.connector.get_connector_integration();

        // let router_resp = services::execute_connector_processing_step::<F, _, Req, Resp>(
        //     &self.handler.state,
        //     connector_integration,
        //     &router_data,
        //     payments::CallConnectorAction::Trigger,
        //     None,
        //     None,
        // )
        // .await
        // .change_context(errors::ApiErrorResponse::InternalServerError)
        // .attach_printable(format!(
        //     "Failed while calling {} at billing processor",
        //     operation_name
        // ))?;

        // match router_resp.response {
        //     Ok(response_data) => Ok(response_data),
        //     Err(err) => Err(errors::ApiErrorResponse::ExternalConnectorError {
        //         code: err.code,
        //         message: err.message,
        //         connector: self.connector_name.to_string(),
        //         status_code: err.status_code,
        //         reason: err.reason,
        //     }
        //     .into()),
        // }

        todo!("Call the connector and return the response")
    }
    fn build_customer_router_data(
        &self,
    ) -> errors::RouterResult<
        hyperswitch_domain_models::router_data::RouterData<
            subscription_types::CreateCustomer,
            subscription_types::CreateCustomerRequest,
            subscription_types::CreateCustomerResponse,
        >,
    > {
        // Build customer creation router data
        todo!("Build customer router data")
    }
    fn build_subscription_router_data(
        &self,
        _customer_id: &common_utils::id_type::CustomerId,
    ) -> errors::RouterResult<
        hyperswitch_domain_models::router_data::RouterData<
            subscription_types::CreateSubscription,
            subscription_types::SubscriptionCreateRequest,
            subscription_types::SubscriptionCreateResponse,
        >,
    > {
        // Build subscription creation router data using customer_id
        todo!("Build subscription router data")
    }
}
