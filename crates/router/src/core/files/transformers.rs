use actix_multipart::Field;
use futures::TryStreamExt;

use crate::types::api;

pub async fn read_string(field: &mut Field) -> Option<String> {
    let bytes = field.try_next().await;
    if let Ok(Some(bytes)) = bytes {
        String::from_utf8(bytes.to_vec()).ok()
    } else {
        None
    }
}

pub async fn get_file_purpose(field: &mut Field) -> Option<api::FilePurpose> {
    let purpose = read_string(field).await;
    match purpose.as_deref() {
        Some("dispute_evidence") => Some(api::FilePurpose::DisputeEvidence),
        _ => None,
    }
}
