use api_models::blocklist;

use crate::types::{storage, transformers::ForeignFrom};

impl ForeignFrom<storage::Blocklist> for blocklist::AddToBlocklistResponse {
    fn foreign_from(from: storage::Blocklist) -> Self {
        Self {
            fingerprint_id: from.fingerprint_id,
            data_kind: from.data_kind,
            created_at: from.created_at,
        }
    }
}
