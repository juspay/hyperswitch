use common_utils::encryption::Encryption;
use diesel::{self, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::hyperswitch_ai_interaction;

#[derive(
    Clone,
    Debug,
    Deserialize,
    Identifiable,
    Queryable,
    Selectable,
    Serialize,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = hyperswitch_ai_interaction, primary_key(id, created_at), check_for_backend(diesel::pg::Pg))]
pub struct HyperswitchAiInteraction {
    pub id: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: Option<String>,
    pub org_id: Option<String>,
    pub role_id: Option<String>,
    pub user_query: Option<Encryption>,
    pub response: Option<Encryption>,
    pub database_query: Option<String>,
    pub interaction_status: Option<String>,
    pub created_at: PrimitiveDateTime,
}

#[derive(router_derive::Setter, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = hyperswitch_ai_interaction)]
pub struct HyperswitchAiInteractionNew {
    pub id: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: Option<String>,
    pub org_id: Option<String>,
    pub role_id: Option<String>,
    pub user_query: Option<Encryption>,
    pub response: Option<Encryption>,
    pub database_query: Option<String>,
    pub interaction_status: Option<String>,
    pub created_at: PrimitiveDateTime,
}
