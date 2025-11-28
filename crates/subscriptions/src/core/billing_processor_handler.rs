use std::str::FromStr;

use api_models::subscription as subscription_types;
use common_enums::{connector_enums, CallConnectorAction};
use common_utils::{ext_traits::ValueExt, pii};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response as errors,
    router_data_v2::flow_common_types::{
        GetSubscriptionEstimateData, GetSubscriptionPlanPricesData, GetSubscriptionPlansData,
        InvoiceRecordBackData, SubscriptionCancelData, SubscriptionCreateData,
        SubscriptionCustomerData, SubscriptionPauseData, SubscriptionResumeData,
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
use hyperswitch_interfaces::{
    api_client, configs::MerchantConnectorAccountType, connector_integration_interface,
};

use crate::{errors::SubscriptionResult, state::SubscriptionState as SessionState};

pub struct BillingHandler {
    pub auth_type: hyperswitch_domain_models::router_data::ConnectorAuthType,
    pub connector_name: connector_enums::Connector,
    pub connector_enum: connector_integration_interface::ConnectorEnum,
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
    ) -> SubscriptionResult<Self> {
        let merchant_connector_id = profile.get_billing_processor_id()?;

        let billing_processor_mca = state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
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
            MerchantConnectorAccountType::DbVal(Box::new(billing_processor_mca.clone()))
                .get_connector_account_details()
                .parse_value("ConnectorAuthType")
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "connector_account_details".to_string(),
                    expected_format: "auth_type and api_key".to_string(),
                })?;

        let connector_enum = state
            .connector_converter
            .get_connector_enum_by_name(&connector_name)
            .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
            .attach_printable(
                "invalid connector name received in billing merchant connector account",
            )?;

        let connector_data = connector_enums::Connector::from_str(connector_name.as_str())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("unable to parse connector name {connector_name:?}"))?;

        let connector_params =
            hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
                &state.conf.connectors,
                connector_data,
            )
            .change_context(errors::ApiErrorResponse::ConfigNotFound)
            .attach_printable(format!(
                "cannot find connector params for this connector {connector_name} in this flow",
            ))?;

        Ok(Self {
            auth_type,
            connector_enum,
            connector_name: connector_data,
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
    ) -> SubscriptionResult<Option<ConnectorCustomerResponseData>> {
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
        let connector_integration = self.connector_enum.get_connector_integration();

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
                connector: self.connector_name.to_string(),
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
    ) -> SubscriptionResult<subscription_response_types::SubscriptionCreateResponse> {
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
        let connector_integration = self.connector_enum.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "create subscription on connector",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
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
    ) -> SubscriptionResult<InvoiceRecordBackResponse> {
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
        let connector_integration = self.connector_enum.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "invoice record back",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
    }

    pub async fn get_subscription_estimate(
        &self,
        state: &SessionState,
        estimate_request: subscription_types::EstimateSubscriptionQuery,
    ) -> SubscriptionResult<subscription_response_types::GetSubscriptionEstimateResponse> {
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
        let connector_integration = self.connector_enum.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "get subscription estimate from connector",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
    }

    pub async fn get_subscription_plans(
        &self,
        state: &SessionState,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> SubscriptionResult<subscription_response_types::GetSubscriptionPlansResponse> {
        let get_plans_request =
            subscription_request_types::GetSubscriptionPlansRequest::new(limit, offset);

        let router_data = self.build_router_data(
            state,
            get_plans_request,
            GetSubscriptionPlansData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;

        let connector_integration = self.connector_enum.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "get subscription plans",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
    }

    pub async fn get_subscription_plan_prices(
        &self,
        state: &SessionState,
        plan_price_id: String,
    ) -> SubscriptionResult<subscription_response_types::GetSubscriptionPlanPricesResponse> {
        let get_plan_prices_request =
            subscription_request_types::GetSubscriptionPlanPricesRequest { plan_price_id };

        let router_data = self.build_router_data(
            state,
            get_plan_prices_request,
            GetSubscriptionPlanPricesData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;

        let connector_integration = self.connector_enum.get_connector_integration();

        let response = self
            .call_connector(
                state,
                router_data,
                "get subscription plan prices",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
    }

    pub async fn pause_subscription_on_connector(
        &self,
        state: &SessionState,
        subscription: &hyperswitch_domain_models::subscription::Subscription,
        request: &subscription_types::PauseSubscriptionRequest,
    ) -> SubscriptionResult<subscription_response_types::SubscriptionPauseResponse> {
        let pause_subscription_request = subscription_request_types::SubscriptionPauseRequest {
            subscription_id: subscription.id.clone(),
            pause_option: request.pause_option.clone(),
            pause_date: request.pause_at,
        };

        let router_data = self.build_router_data(
            state,
            pause_subscription_request,
            SubscriptionPauseData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;
        let connector_integration = self.connector_enum.get_connector_integration();
        let response = self
            .call_connector(
                state,
                router_data,
                "pause subscription",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
    }

    pub async fn resume_subscription_on_connector(
        &self,
        state: &SessionState,
        subscription: &hyperswitch_domain_models::subscription::Subscription,
        request: &subscription_types::ResumeSubscriptionRequest,
    ) -> SubscriptionResult<subscription_response_types::SubscriptionResumeResponse> {
        let resume_subscription_request = subscription_request_types::SubscriptionResumeRequest {
            subscription_id: subscription.id.clone(),
            resume_date: request.resume_date,
            charges_handling: request.charges_handling.clone(),
            resume_option: request.resume_option.clone(),
            unpaid_invoices_handling: request.unpaid_invoices_handling.clone(),
        };

        let router_data = self.build_router_data(
            state,
            resume_subscription_request,
            SubscriptionResumeData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;
        let connector_integration = self.connector_enum.get_connector_integration();
        let response = self
            .call_connector(
                state,
                router_data,
                "resume subscription",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
    }

    pub async fn cancel_subscription_on_connector(
        &self,
        state: &SessionState,
        subscription: &hyperswitch_domain_models::subscription::Subscription,
        request: &subscription_types::CancelSubscriptionRequest,
    ) -> SubscriptionResult<subscription_response_types::SubscriptionCancelResponse> {
        let cancel_subscription_request = subscription_request_types::SubscriptionCancelRequest {
            subscription_id: subscription.id.clone(),
            cancel_date: request.cancel_at,
            account_receivables_handling: request.account_receivables_handling.clone(),
            cancel_option: request.cancel_option.clone(),
            cancel_reason_code: request.cancel_reason_code.clone(),
            credit_option_for_current_term_charges: request
                .credit_option_for_current_term_charges
                .clone(),
            refundable_credits_handling: request.refundable_credits_handling.clone(),
            unbilled_charges_option: request.unbilled_charges_option.clone(),
        };

        let router_data = self.build_router_data(
            state,
            cancel_subscription_request,
            SubscriptionCancelData {
                connector_meta_data: self.connector_metadata.clone(),
            },
        )?;
        let connector_integration = self.connector_enum.get_connector_integration();
        let response = self
            .call_connector(
                state,
                router_data,
                "cancel subscription",
                connector_integration,
            )
            .await?;
        self.handle_connector_response(response)
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
        connector_integration: connector_integration_interface::BoxedConnectorIntegrationInterface<
            F,
            ResourceCommonData,
            Req,
            Resp,
        >,
    ) -> SubscriptionResult<Result<Resp, hyperswitch_domain_models::router_data::ErrorResponse>>
    where
        F: Clone + std::fmt::Debug + 'static,
        Req: Clone + std::fmt::Debug + 'static,
        Resp: Clone + std::fmt::Debug + 'static,
        ResourceCommonData:
            connector_integration_interface::RouterDataConversion<F, Req, Resp> + Clone + 'static,
    {
        let old_router_data = ResourceCommonData::to_old_router_data(router_data).change_context(
            errors::ApiErrorResponse::SubscriptionError {
                operation: { operation_name.to_string() },
            },
        )?;

        let router_resp = api_client::execute_connector_processing_step(
            state,
            connector_integration,
            &old_router_data,
            CallConnectorAction::Trigger,
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
    ) -> SubscriptionResult<
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

    fn handle_connector_response<T>(
        &self,
        response: Result<T, hyperswitch_domain_models::router_data::ErrorResponse>,
    ) -> SubscriptionResult<T> {
        match response {
            Ok(resp) => Ok(resp),
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
}
