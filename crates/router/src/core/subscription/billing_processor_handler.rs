use std::str::FromStr;

use common_enums::connector_enums;
use common_utils::{ext_traits::ValueExt, pii};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        GetSubscriptionEstimateData, GetSubscriptionPlanPricesData, GetSubscriptionPlansData,
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

use super::errors;
use crate::{
    core::{payments as payments_core, subscription::subscription_types},
    routes::SessionState,
    services,
    types::api as api_types,
};

pub struct BillingHandler {
    pub auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType,
    pub connector_data: api_types::ConnectorData,
    pub connector_params: hyperswitch_domain_models::connector_endpoints::ConnectorParams,
    pub connector_metadata: Option<pii::SecretSerdeValue>,
    pub merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
}

#[allow(clippy::todo)]
impl BillingHandler {
    pub async fn create(
        state: &SessionState,
        merchant_account: &hyperswitch_domain_models::merchant_account::MerchantAccount,
        key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
        profile: hyperswitch_domain_models::business_profile::Profile,
    ) -> errors::RouterResult<Self> {
        let merchant_connector_id = profile.get_billing_processor_id()?;

        let billing_processor_mca = state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &(state).into(),
                merchant_account.get_id(),
                &merchant_connector_id,
                key_store,
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
            merchant_connector_id,
        })
    }
    pub async fn create_customer_on_connector(
        &self,
        state: &SessionState,
        customer: hyperswitch_domain_models::customer::Customer,
        customer_id: common_utils::id_type::CustomerId,
        billing_address: Option<api_models::payments::Address>,
        payment_method_data: Option<api_models::payments::PaymentMethodData>,
    ) -> errors::RouterResult<Option<ConnectorCustomerResponseData>> {
        let connector_customer_map = customer.get_connector_customer_map();
        if connector_customer_map.contains_key(&self.merchant_connector_id) {
            // Customer already exists on the connector, no need to create again
            return Ok(None);
        }
        let customer_req = ConnectorCustomerData {
            email: customer.email.clone().map(pii::Email::from),
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
                    Ok(Some(customer_response))
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
        subscription: hyperswitch_domain_models::subscription::Subscription,
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
    #[allow(clippy::too_many_arguments)]
    pub async fn record_back_to_billing_processor(
        &self,
        state: &SessionState,
        invoice_id: common_utils::id_type::InvoiceId,
        payment_id: common_utils::id_type::PaymentId,
        payment_status: common_enums::AttemptStatus,
        amount: common_utils::types::MinorUnit,
        currency: common_enums::Currency,
        payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> errors::RouterResult<InvoiceRecordBackResponse> {
        let invoice_record_back_req = InvoiceRecordBackRequest {
            amount,
            currency,
            payment_method_type,
            attempt_status: payment_status,
            merchant_reference_id: common_utils::id_type::PaymentReferenceId::from_str(
                invoice_id.get_string_repr(),
            )
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "invoice_id",
            })?,
            connector_params: self.connector_params.clone(),
            connector_transaction_id: Some(common_utils::types::ConnectorTransactionId::TxnId(
                payment_id.get_string_repr().to_string(),
            )),
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

    pub async fn get_subscription_estimate(
        &self,
        state: &SessionState,
        estimate_request: subscription_types::EstimateSubscriptionQuery,
    ) -> errors::RouterResult<subscription_response_types::GetSubscriptionEstimateResponse> {
        let estimate_req = subscription_request_types::GetSubscriptionEstimateRequest {
            price_id: estimate_request.item_price_id.clone(),
        };

        let router_data = self.build_router_data(
            state,
            estimate_req,
            GetSubscriptionEstimateData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;
        let connector_integration = self.connector_data.connector.get_connector_integration();

        let response = Box::pin(self.call_connector(
            state,
            router_data,
            "get subscription estimate from connector",
            connector_integration,
        ))
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

    pub async fn get_subscription_plans(
        &self,
        state: &SessionState,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> errors::RouterResult<subscription_response_types::GetSubscriptionPlansResponse> {
        let get_plans_request =
            subscription_request_types::GetSubscriptionPlansRequest::new(limit, offset);

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
    ) -> errors::RouterResult<subscription_response_types::GetSubscriptionPlanPricesResponse> {
        let get_plan_prices_request =
            subscription_request_types::GetSubscriptionPlanPricesRequest { plan_price_id };

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
        connector_integration: hyperswitch_interfaces::connector_integration_interface::BoxedConnectorIntegrationInterface<
        F,
        ResourceCommonData,
        Req,
        Resp,
    >,
    ) -> errors::RouterResult<Result<Resp, hyperswitch_domain_models::router_data::ErrorResponse>>
    where
        // Core data requirements
        Req: Clone + std::fmt::Debug + Send + Sync + 'static,
        Resp: Clone + std::fmt::Debug + Send + Sync + 'static,
        ResourceCommonData:
            hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    F,
                    Req,
                    Resp,
                > + Clone
                + Send
                + Sync
                + 'static,

        F: Clone
            + std::fmt::Debug
            + Send
            + Sync
            + 'static
            + hyperswitch_interfaces::unified_connector_service::UnifiedConnectorServiceFlow<
                F,
                Req,
                Resp,
            >,

        // Also this bound ensures the connector types align
        dyn hyperswitch_interfaces::api::Connector + Sync:
            hyperswitch_interfaces::api::ConnectorIntegration<F, Req, Resp>,
    {
        let old_router_data = ResourceCommonData::to_old_router_data(router_data).change_context(
            errors::ApiErrorResponse::SubscriptionError {
                operation: operation_name.to_string(),
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
