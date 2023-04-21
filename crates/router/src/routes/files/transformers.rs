use std::num::TryFromIntError;

use actix_multipart::Multipart;
use actix_web::web::Bytes;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use futures::{StreamExt, TryStreamExt};

use crate::{
    core::{errors, files::helpers},
    types::api::files::{self, CreateFileRequest},
};

pub async fn get_create_file_request(
    mut payload: Multipart,
) -> CustomResult<CreateFileRequest, errors::ApiErrorResponse> {
    let mut option_purpose: Option<files::FilePurpose> = None;
    let mut dispute_id: Option<String> = None;

    let mut file_name: Option<String> = None;
    let mut file_content: Option<Vec<Bytes>> = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name();
        // Parse the different parameters expected in the multipart request
        match field_name {
            Some("purpose") => {
                option_purpose = helpers::get_file_purpose(&mut field).await;
            }
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
            // Can ignore other params
            _ => (),
        }
    }
    let purpose = match option_purpose {
        Some(valid_purpose) => valid_purpose,
        None => Err(errors::ApiErrorResponse::MissingFilePurpose)
            .into_report()
            .attach_printable("Missing file purpose in the request")?,
    };
    let file = match file_content {
        Some(valid_file_content) => valid_file_content.concat().to_vec(),
        None => Err(errors::ApiErrorResponse::MissingFile)
            .into_report()
            .attach_printable("Missing / Invalid file in the request")?,
    };
    //Get and validate file size
    let file_size_result: Result<i32, TryFromIntError> = file.len().try_into();
    let file_size = match file_size_result {
        Ok(valid_file_size) => valid_file_size,
        Err(err) => Err(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable(format!("{}{}", "File size error: ", err))?,
    };
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
    Ok(CreateFileRequest {
        file,
        file_name,
        file_size,
        file_type,
        purpose,
        dispute_id,
    })
}
