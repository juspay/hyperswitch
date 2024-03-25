use actix_multipart::{Field, Multipart};
use actix_web::web::Bytes;
use common_utils::{errors::CustomResult, ext_traits::StringExt, fp_utils};
use error_stack::ResultExt;
use futures::{StreamExt, TryStreamExt};

use crate::{
    core::{errors, files::helpers},
    types::api::{disputes, files},
    utils::OptionExt,
};

pub async fn parse_evidence_type(
    field: &mut Field,
) -> CustomResult<Option<disputes::EvidenceType>, errors::ApiErrorResponse> {
    let purpose = helpers::read_string(field).await;
    match purpose {
        Some(evidence_type) => Ok(Some(
            evidence_type
                .parse_enum("Evidence Type")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing evidence type")?,
        )),
        _ => Ok(None),
    }
}

pub async fn get_attach_evidence_request(
    mut payload: Multipart,
) -> CustomResult<disputes::AttachEvidenceRequest, errors::ApiErrorResponse> {
    let mut option_evidence_type: Option<disputes::EvidenceType> = None;
    let mut dispute_id: Option<String> = None;

    let mut file_name: Option<String> = None;
    let mut file_content: Option<Vec<Bytes>> = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name();
        // Parse the different parameters expected in the multipart request
        match field_name {
            Some("file") => {
                file_name = content_disposition.get_filename().map(String::from);
                //Collect the file content and throw error if something fails
                let mut file_data = Vec::new();
                let mut stream = field.into_stream();
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => file_data.push(bytes),
                        Err(err) => Err(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable_lazy(|| format!("File parsing error: {err}"))?,
                    }
                }
                file_content = Some(file_data)
            }
            Some("dispute_id") => {
                dispute_id = helpers::read_string(&mut field).await;
            }
            Some("evidence_type") => {
                option_evidence_type = parse_evidence_type(&mut field).await?;
            }
            // Can ignore other params
            _ => (),
        }
    }
    let evidence_type = option_evidence_type.get_required_value("evidence_type")?;
    let file = file_content.get_required_value("file")?.concat().to_vec();
    //Get and validate file size
    let file_size = i32::try_from(file.len())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("File size error")?;
    // Check if empty file and throw error
    fp_utils::when(file_size <= 0, || {
        Err(errors::ApiErrorResponse::MissingFile)
            .attach_printable("Missing / Invalid file in the request")
    })?;
    // Get file mime type using 'infer'
    let kind = infer::get(&file).ok_or(errors::ApiErrorResponse::MissingFileContentType)?;
    let file_type = kind
        .mime_type()
        .parse::<mime::Mime>()
        .change_context(errors::ApiErrorResponse::MissingFileContentType)
        .attach_printable("File content type error")?;
    let create_file_request = files::CreateFileRequest {
        file,
        file_name,
        file_size,
        file_type,
        purpose: files::FilePurpose::DisputeEvidence,
        dispute_id,
    };
    Ok(disputes::AttachEvidenceRequest {
        evidence_type,
        create_file_request,
    })
}
