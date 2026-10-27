//! Access-token handling for FRM providers, the FRM counterpart of
//! `payments::access_token` and `payouts::access_token`.

use std::str::FromStr;

use common_enums::connector_enums::Connector;
use hyperswitch_domain_models::router_data::RouterData;

use crate::services::logger;

/// Attach the access token a bearer-authenticated FRM provider needs before the
/// risk check. Providers outside `Connector::supports_access_token_for_frm`
/// (static-key ones such as nSure) are left untouched, so no token call is made
/// for them.
pub fn add_access_token_for_frm<F, Req, Res>(router_data: &mut RouterData<F, Req, Res>) {
    let connector_supports_access_token = Connector::from_str(&router_data.connector)
        .is_ok_and(|connector| connector.supports_access_token_for_frm());

    if !connector_supports_access_token || router_data.access_token.is_some() {
        return;
    }

    // Fetching the token (Redis cache first, then UCS `create_access_token`, as
    // payments does) will be implemented based on requirement, once a
    // bearer-authenticated FRM provider is onboarded.
    logger::warn!(
        connector = %router_data.connector,
        "Access token fetch is not implemented for FRM providers; continuing without one"
    );
}
