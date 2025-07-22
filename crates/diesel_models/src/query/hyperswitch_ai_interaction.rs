use crate::{hyperswitch_ai_interaction::*, query::generics, PgPooledConn, StorageResult};

impl HyperswitchAiInteractionNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<HyperswitchAiInteraction> {
        generics::generic_insert(conn, self).await
    }
}
