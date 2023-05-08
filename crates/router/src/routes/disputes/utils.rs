use actix_multipart::{Field, Multipart};
use actix_web::web::Bytes;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use futures::{StreamExt, TryStreamExt};

use crate::{
    core::{errors, files::helpers},
    types::api::{disputes, files},
    utils::OptionExt,
};

pub async fn parse_evidence_type(field: &mut Field) -> Option<disputes::EvidenceType> {
    let purpose = helpers::read_string(field).await;
    match purpose.as_deref() {
        Some("cancellation_policy") => Some(disputes::EvidenceType::CancellationPolicy),
        Some("customer_communication") => Some(disputes::EvidenceType::CustomerCommunication),
        Some("customer_signature") => Some(disputes::EvidenceType::CustomerSignature),
        Some("receipt") => Some(disputes::EvidenceType::Receipt),
        Some("refund_policy") => Some(disputes::EvidenceType::RefundPolicy),
        Some("service_documentation") => Some(disputes::EvidenceType::ServiceDocumentation),
        Some("shipping_documentaion") => Some(disputes::EvidenceType::ShippingDocumentaion),
        Some("invoice_showing_distinct_transactions") => {
            Some(disputes::EvidenceType::InvoiceShowingDistinctTransactions)
        }
        Some("recurring_transaction_agreement") => {
            Some(disputes::EvidenceType::RecurringTransactionAgreement)
        }
        Some("uncategorized_file") => Some(disputes::EvidenceType::UncategorizedFile),
        _ => None,
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
                            .into_report()
                            .attach_printable(format!("{}{}", "File parsing error: ", err))?,
                    }
                }
                file_content = Some(file_data)
            }
            Some("dispute_id") => {
                dispute_id = helpers::read_string(&mut field).await;
            }
            Some("evidence_type") => {
                option_evidence_type = parse_evidence_type(&mut field).await;
            }
            // Can ignore other params
            _ => (),
        }
    }
    let evidence_type = option_evidence_type.get_required_value("evidence_type")?;
    let file = match file_content {
        Some(valid_file_content) => valid_file_content.concat().to_vec(),
        None => Err(errors::ApiErrorResponse::MissingFile)
            .into_report()
            .attach_printable("Missing / Invalid file in the request")?,
    };
    //Get and validate file size
    let file_size: i32 = file
        .len()
        .try_into()
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("File size error")?;
    // Check if empty file and throw error
    if file_size <= 0 {
        Err(errors::ApiErrorResponse::MissingFile)
            .into_report()
            .attach_printable("Missing / Invalid file in the request")?
    }
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
