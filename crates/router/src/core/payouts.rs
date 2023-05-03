use common_utils::ext_traits::AsyncExt;
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt},
    routes::{metrics, AppState},
    types::{api::payouts, storage},
};

#[instrument(skip_all)]
pub async fn payout_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: payouts::PayoutCreateRequest,
) -> RouterResponse<payouts::PayoutCreateResponse> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let payout_create = storage::PayoutCreateNew::default();
    db.insert
    //if eligible
}
