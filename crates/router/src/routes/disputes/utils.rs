use actix_multipart::{Field, Multipart};
use actix_web::web::Bytes;
use common_utils::{errors::CustomResult, ext_traits::StringExt, fp_utils};
use error_stack::{IntoReport, ResultExt};
use futures::{StreamExt, TryStreamExt};

use crate::{
    core::{errors, files::helpers},
    types::api::{disputes, files},
    utils::OptionExt,
};

/// Asynchronously parses the evidence type from a given field and returns a result containing the parsed evidence type or an API error response.
///
/// # Arguments
///
/// * `field` - A mutable reference to the Field object from which the evidence type will be parsed.
///
/// # Returns
///
/// A custom result containing either the parsed evidence type as Some(disputes::EvidenceType) or an API error response as errors::ApiErrorResponse.
///
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

/// This method takes a multipart payload as input and parses it to extract the necessary parameters for attaching evidence to a dispute. It extracts the file, dispute_id, and evidence_type from the payload, validates the file size and type, and creates a file request. Finally, it returns an AttachEvidenceRequest object containing the evidence type and the create file request.
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
                            .into_report()
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
    let file_size: i32 = file
        .len()
        .try_into()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("File size error")?;
    // Check if empty file and throw error
    fp_utils::when(file_size <= 0, || {
        Err(errors::ApiErrorResponse::MissingFile)
            .into_report()
            .attach_printable("Missing / Invalid file in the request")
    })?;
    // Get file mime type using 'infer'
    let kind = infer::get(&file).ok_or(errors::ApiErrorResponse::MissingFileContentType)?;
    let file_type = kind
        .mime_type()
        .parse::<mime::Mime>()
        .into_report()
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
