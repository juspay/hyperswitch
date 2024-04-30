pub mod fraud_check_post;
pub mod fraud_check_pre;
use async_trait::async_trait;
use common_enums::FrmSuggestion;
use error_stack::{report, ResultExt};

pub use self::{fraud_check_post::FraudCheckPost, fraud_check_pre::FraudCheckPre};
use super::{
    types::{ConnectorDetailsCore, FrmConfigsObject, PaymentToFrmData},
    FrmData,
};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments,
    },
    db::StorageInterface,
    routes::{app::ReqState, AppState},
    types::{domain, fraud_check::FrmRouterData},
};

pub type BoxedFraudCheckOperation<F> = Box<dyn FraudCheckOperation<F> + Send + Sync>;

pub trait FraudCheckOperation<F>: Send + std::fmt::Debug {
    fn to_get_tracker(&self) -> RouterResult<&(dyn GetTracker<PaymentToFrmData> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("get tracker interface not found for {self:?}"))
    }
    fn to_domain(&self) -> RouterResult<&(dyn Domain<F>)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("domain interface not found for {self:?}"))
    }
    fn to_update_tracker(&self) -> RouterResult<&(dyn UpdateTracker<FrmData, F> + Send + Sync)> {
        Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable_lazy(|| format!("get tracker interface not found for {self:?}"))
    }
}

#[async_trait]
pub trait GetTracker<D>: Send {
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: D,
        frm_connector_details: ConnectorDetailsCore,
    ) -> RouterResult<Option<FrmData>>;
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait Domain<F>: Send + Sync {
    async fn post_payment_frm<'a>(
        &'a self,
        state: &'a AppState,
        req_state: ReqState,
        payment_data: &mut payments::PaymentData<F>,
        frm_data: &mut FrmData,
        merchant_account: &domain::MerchantAccount,
        customer: &Option<domain::Customer>,
        key_store: domain::MerchantKeyStore,
    ) -> RouterResult<Option<FrmRouterData>>
    where
        F: Send + Clone;

    async fn pre_payment_frm<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut payments::PaymentData<F>,
        frm_data: &mut FrmData,
        merchant_account: &domain::MerchantAccount,
        customer: &Option<domain::Customer>,
        key_store: domain::MerchantKeyStore,
    ) -> RouterResult<FrmRouterData>
    where
        F: Send + Clone;

    // To execute several tasks conditionally based on the result of post_flow.
    // Eg: If the /sale(post flow) is returning the transaction as fraud we can execute refund in post task
    #[allow(clippy::too_many_arguments)]
    async fn execute_post_tasks(
        &self,
        _state: &AppState,
        _req_state: ReqState,
        frm_data: &mut FrmData,
        _merchant_account: &domain::MerchantAccount,
        _frm_configs: FrmConfigsObject,
        _frm_suggestion: &mut Option<FrmSuggestion>,
        _key_store: domain::MerchantKeyStore,
        _payment_data: &mut payments::PaymentData<F>,
        _customer: &Option<domain::Customer>,
        _should_continue_capture: &mut bool,
    ) -> RouterResult<Option<FrmData>>
    where
        F: Send + Clone,
    {
        return Ok(Some(frm_data.to_owned()));
    }
}

#[async_trait]
pub trait UpdateTracker<D, F: Clone>: Send {
    async fn update_tracker<'b>(
        &'b self,
        db: &dyn StorageInterface,
        frm_data: D,
        payment_data: &mut payments::PaymentData<F>,
        _frm_suggestion: Option<FrmSuggestion>,
        frm_router_data: FrmRouterData,
    ) -> RouterResult<D>;
}
