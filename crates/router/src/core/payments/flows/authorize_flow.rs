use async_trait::async_trait;
use common_utils::{ext_traits::ValueExt, pii};
use error_stack::{report, ResultExt};
use masking::ExposeInterface;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        mandate, payment_methods,
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{
        self,
        api::{self, PaymentMethodCreateExt},
        domain::{customer, merchant_account},
        storage,
    },
    utils::OptionExt,
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::Authorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &merchant_account::MerchantAccount,
    ) -> RouterResult<
        types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        transformers::construct_payment_router_data::<api::Authorize, types::PaymentsAuthorizeData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
        )
        .await
    }
}
#[async_trait]
impl Feature<api::Authorize, types::PaymentsAuthorizeData> for types::PaymentsAuthorizeRouterData {
    async fn decide_flows<'a>(
        mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        customer: &Option<customer::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &merchant_account::MerchantAccount,
    ) -> RouterResult<Self> {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                Some(true),
                call_connector_action,
                merchant_account,
            )
            .await;

        metrics::PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics

        resp
    }

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &merchant_account::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn add_payment_method_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
    ) -> RouterResult<Option<String>> {
        add_payment_method_token(state, connector, tokenization_action, self).await
    }
}

impl types::PaymentsAuthorizeRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b mut self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<customer::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &merchant_account::MerchantAccount,
    ) -> RouterResult<Self> {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                connector_integration
                    .execute_pretasks(self, state)
                    .await
                    .map_err(|error| error.to_payment_failed_response())?;
                logger::debug!(completed_pre_tasks=?true);
                if self.should_proceed_with_authorize() {
                    self.decide_authentication_type();
                    logger::debug!(auth_type=?self.auth_type);
                    let resp = services::execute_connector_processing_step(
                        state,
                        connector_integration,
                        self,
                        call_connector_action,
                    )
                    .await
                    .map_err(|error| error.to_payment_failed_response())?;

                    let pm_id = save_payment_method(
                        state,
                        connector,
                        resp.to_owned(),
                        maybe_customer,
                        merchant_account,
                    )
                    .await?;

                    Ok(mandate::mandate_procedure(state, resp, maybe_customer, pm_id).await?)
                } else {
                    Ok(self.clone())
                }
            }
            _ => Ok(self.clone()),
        }
    }

    fn decide_authentication_type(&mut self) {
        if self.auth_type == storage_models::enums::AuthenticationType::ThreeDs
            && !self.request.enrolled_for_3ds
        {
            self.auth_type = storage_models::enums::AuthenticationType::NoThreeDs
        }
    }

    /// to decide if we need to proceed with authorize or not, Eg: If any of the pretask returns `redirection_response` then we should not proceed with authorize call
    fn should_proceed_with_authorize(&self) -> bool {
        match &self.response {
            Ok(types::PaymentsResponseData::TransactionResponse {
                redirection_data, ..
            }) => !redirection_data.is_some(),
            _ => true,
        }
    }
}

pub async fn save_payment_method<F: Clone, FData>(
    state: &AppState,
    connector: &api::ConnectorData,
    resp: types::RouterData<F, FData, types::PaymentsResponseData>,
    maybe_customer: &Option<storage::Customer>,
    merchant_account: &storage::MerchantAccount,
) -> RouterResult<Option<String>>
where
    FData: mandate::MandateBehaviour,
{
    let db = &*state.store;
    let token_store = state
        .conf
        .tokenization
        .0
        .get(&connector.connector_name.to_string())
        .map(|token_filter| token_filter.long_lived_token)
        .unwrap_or(false);

    let connector_token = if token_store {
        let token = resp
            .payment_method_token
            .to_owned()
            .get_required_value("payment_token")?;
        Some((connector, token))
    } else {
        None
    };

    let pm_id = if resp.request.get_setup_future_usage().is_some() {
        let customer = maybe_customer.to_owned().get_required_value("customer")?;
        let payment_method_create_request = helpers::get_payment_method_create_request(
            Some(&resp.request.get_payment_method_data()),
            Some(resp.payment_method),
            &customer,
        )
        .await?;
        let merchant_id = &merchant_account.merchant_id;

        let locker_response = save_in_locker(
            state,
            merchant_account,
            payment_method_create_request.to_owned(),
        )
        .await?;
        let is_duplicate = locker_response.1;

        if is_duplicate {
            let existing_pm = db
                .find_payment_method(&locker_response.0.payment_method_id)
                .await;
            match existing_pm {
                Ok(pm) => {
                    let pm_metadata =
                        create_payment_method_metadata(pm.metadata.as_ref(), connector_token)?;
                    if let Some(metadata) = pm_metadata {
                        payment_methods::cards::update_payment_method(db, pm, metadata)
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to add payment method in db")?;
                    };
                }
                Err(error) => {
                    match error.current_context() {
                        errors::StorageError::DatabaseError(err) => match err.current_context() {
                            storage_models::errors::DatabaseError::NotFound => {
                                let pm_metadata =
                                    create_payment_method_metadata(None, connector_token)?;
                                payment_methods::cards::create_payment_method(
                                    db,
                                    &payment_method_create_request,
                                    &customer.customer_id,
                                    &locker_response.0.payment_method_id,
                                    merchant_id,
                                    pm_metadata,
                                )
                                .await
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Failed to add payment method in db")
                            }
                            _ => Err(report!(errors::ApiErrorResponse::InternalServerError)),
                        },
                        _ => Err(report!(errors::ApiErrorResponse::InternalServerError)),
                    }?;
                }
            };
        } else {
            let pm_metadata = create_payment_method_metadata(None, connector_token)?;
            payment_methods::cards::create_payment_method(
                db,
                &payment_method_create_request,
                &customer.customer_id,
                &locker_response.0.payment_method_id,
                merchant_id,
                pm_metadata,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add payment method in db")?;
        };
        Some(locker_response.0.payment_method_id)
    } else {
        None
    };
    Ok(pm_id)
}

pub async fn save_in_locker(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(api_models::payment_methods::PaymentMethodResponse, bool)> {
    payment_method_request.validate()?;
    let merchant_id = &merchant_account.merchant_id;
    let customer_id = payment_method_request
        .customer_id
        .clone()
        .get_required_value("customer_id")?;
    match payment_method_request.card.clone() {
        Some(card) => payment_methods::cards::add_card_to_locker(
            state,
            payment_method_request,
            card,
            customer_id,
            merchant_account,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Add Card Failed"),
        None => {
            let pm_id = common_utils::generate_id(crate::consts::ID_LENGTH, "pm");
            let payment_method_response = api::PaymentMethodResponse {
                merchant_id: merchant_id.to_string(),
                customer_id: Some(customer_id),
                payment_method_id: pm_id,
                payment_method: payment_method_request.payment_method,
                payment_method_type: payment_method_request.payment_method_type,
                card: None,
                metadata: None,
                created: Some(common_utils::date_time::now()),
                recurring_enabled: false,           //[#219]
                installment_payment_enabled: false, //[#219]
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219]
            };
            Ok((payment_method_response, false))
        }
    }
}

pub fn create_payment_method_metadata(
    metadata: Option<&pii::SecretSerdeValue>,
    connector_token: Option<(&api::ConnectorData, String)>,
) -> RouterResult<Option<serde_json::Value>> {
    let mut meta = match metadata {
        None => serde_json::Map::new(),
        Some(meta) => {
            let metadata = meta.clone().expose();
            let existing_metadata: serde_json::Map<String, serde_json::Value> = metadata
                .parse_value("Map<String, Value>")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse the metadata")?;
            existing_metadata
        }
    };
    Ok(connector_token.and_then(|connector_and_token| {
        meta.insert(
            connector_and_token.0.connector_name.to_string(),
            serde_json::Value::String(connector_and_token.1),
        )
    }))
}

pub enum Action {
    Update,
    Insert,
    Skip,
}

impl mandate::MandateBehaviour for types::PaymentsAuthorizeData {
    fn get_amount(&self) -> i64 {
        self.amount
    }
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }
    fn get_setup_future_usage(&self) -> Option<storage_models::enums::FutureUsage> {
        self.setup_future_usage
    }
    fn get_setup_mandate_details(&self) -> Option<&api_models::payments::MandateData> {
        self.setup_mandate_details.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: api_models::payments::MandateIds) {
        self.mandate_id = Some(new_mandate_id);
    }
}

pub async fn add_payment_method_token<F: Clone>(
    state: &AppState,
    connector: &api::ConnectorData,
    tokenization_action: &payments::TokenizationAction,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
) -> RouterResult<Option<String>> {
    match tokenization_action {
        payments::TokenizationAction::TokenizeInConnector => {
            let connector_integration: services::BoxedConnectorIntegration<
                '_,
                api::PaymentMethodToken,
                types::PaymentMethodTokenizationData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();

            let pm_token_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
                Err(types::ErrorResponse::default());

            let pm_token_request_data =
                types::PaymentMethodTokenizationData::try_from(router_data.request.to_owned())?;

            let pm_token_router_data = payments::helpers::router_data_type_conversion::<
                _,
                api::PaymentMethodToken,
                _,
                _,
                _,
                _,
            >(
                router_data.clone(),
                pm_token_request_data,
                pm_token_response_data,
            );
            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &pm_token_router_data,
                payments::CallConnectorAction::Trigger,
            )
            .await
            .map_err(|error| error.to_payment_failed_response())?;

            let pm_token = match resp.response {
                Ok(response) => match response {
                    types::PaymentsResponseData::TokenizationResponse { token } => Some(token),
                    _ => None,
                },
                Err(err) => {
                    logger::debug!(payment_method_tokenization_error=?err);
                    None
                }
            };
            Ok(pm_token)
        }
        _ => Ok(None),
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
        })
    }
}
