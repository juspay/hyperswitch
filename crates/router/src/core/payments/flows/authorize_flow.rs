use async_trait::async_trait;
use error_stack::ResultExt;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        mandate, payment_methods,
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::{metrics, AppState},
    services,
    types::{self, api, storage},
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
        merchant_account: &storage::MerchantAccount,
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
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
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
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }
}

impl types::PaymentsAuthorizeRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b mut self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
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
                self.decide_authentication_type();
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
    let tokenization_connector_check = state
        .conf
        .tokenization
        .0
        .get(&connector.connector_name.to_string());
    let token_store = match tokenization_connector_check {
        Some(token_filter) => token_filter.long_lived_token,
        None => false,
    };
    let connector_token = if token_store {
        let token = resp
            .payment_token
            .to_owned()
            .get_required_value("payment_token")?;
        Some((connector, token))
    } else {
        None
    };

    let payment_method_request = helpers::call_payment_method(
        Some(&resp.request.get_payment_method_data()),
        Some(resp.payment_method),
        maybe_customer,
    )
    .await?;

    let pm_id = if resp.request.get_setup_future_usage().is_some() || connector_token.is_some() {
        let merchant_id = &merchant_account.merchant_id;
        let customer_id = payment_method_request
            .customer_id
            .clone()
            .get_required_value("customer_id")?;

        let locker_pm_id = if resp.request.get_setup_future_usage().is_some() {
            let response =
                save_in_locker(state, merchant_account, payment_method_request.to_owned()).await?;
            Some(response.payment_method_id)
        } else {
            None
        };

        let pm_action = match resp.payment_method_id {
            Some(pm_id) => {
                if connector_token.is_some() {
                    let pm_data = Some(db.find_payment_method(&pm_id).await.map_err(|error| {
                        error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
                    })?);

                    (Action::Update, pm_data, pm_id)
                } else {
                    (Action::Skip, None, pm_id)
                }
            }
            None => {
                let pm_id = common_utils::generate_id(common_utils::consts::ID_LENGTH, "pm");
                (Action::Insert, None, pm_id)
            }
        };
        match pm_action {
            (Action::Insert, _, id) => {
                payment_methods::cards::create_payment_method(
                    db,
                    &payment_method_request,
                    &customer_id,
                    &id,
                    merchant_id,
                    connector_token,
                    locker_pm_id,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to add payment method in db")?;
                Some(id)
            }
            (Action::Update, pm_data, id) => {
                let pm = pm_data.get_required_value("pm_data")?;
                payment_methods::cards::update_payment_method(db, connector_token, pm)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add payment method in db")?;
                Some(id)
            }
            (Action::Skip, _, id) => Some(id),
        }
    } else {
        None
    };
    Ok(pm_id)
}

pub async fn save_in_locker(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<api_models::payment_methods::PaymentMethodResponse> {
    let resp =
        payment_methods::cards::add_payment_method(state, payment_method_request, merchant_account)
            .await
            .attach_printable("Error on adding payment method")?;
    match resp {
        crate::services::ApplicationResponse::Json(payment_method) => Ok(payment_method),
        _ => Err(
            error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error on adding payment method"),
        ),
    }
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
