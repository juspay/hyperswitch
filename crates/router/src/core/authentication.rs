pub mod authn;
pub mod post_authn;
pub mod pre_authn;
pub mod types;
pub(crate) mod utils;

use api_models::payments::PaymentMethodData;
use cards::CardNumber;

use crate::{
    core::payments,
    errors::RouterResult,
    types::{api::ConnectorCallType, domain},
    AppState,
};

pub async fn call_payment_3ds_service<F: Send + Clone>(
    state: &AppState,
    payment_data: &mut payments::PaymentData<F>,
    should_continue_confirm_transaction: &mut bool,
    connector_call_type: &ConnectorCallType,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<()> {
    let is_pre_authn_call = payment_data.authentication.is_none();
    let separate_authentication_requested = payment_data
        .payment_attempt
        .external_3ds_authentication_requested
        .unwrap_or(false);
    let connector_supports_separate_authn = utils::is_separate_authn_supported(connector_call_type);
    let card_number = payment_data.payment_method_data.as_ref().and_then(|pmd| {
        if let PaymentMethodData::Card(card) = pmd {
            Some(card.card_number.clone())
        } else {
            None
        }
    });
    if is_pre_authn_call {
        if separate_authentication_requested && connector_supports_separate_authn {
            if let Some(card_number) = card_number {
                let connector_account_for_3ds = "3d_secure_io".to_string();
                pre_authn::execute_pre_auth_flow(
                    state,
                    types::AuthenthenticationFlowInput::PaymentAuthNFlow {
                        payment_data,
                        should_continue_confirm_transaction,
                        card_number,
                    },
                    connector_account_for_3ds,
                    merchant_account,
                )
                .await;
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}

async fn call_payment_method_3ds_service(_card_number: CardNumber) -> RouterResult<()> {
    Ok(())
}
