use std::{
    fs::{remove_file, File},
    io::{Read, Write},
    path::PathBuf,
};

use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};

use crate::{core::errors, env};

pub fn get_file_path(file_key: String) -> PathBuf {
    let mut file_path = PathBuf::new();
    file_path.push(env::workspace_path());
    file_path.push("files");
    file_path.push(file_key);
    file_path
}

pub fn save_file_to_fs(
    file_key: String,
    file_data: Vec<u8>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let file_path = get_file_path(file_key);
    let mut file = File::create(file_path)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to create file")?;
    file.write_all(&file_data)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while writing into file")?;
    Ok(())
}

pub fn delete_file_from_fs(file_key: String) -> CustomResult<(), errors::ApiErrorResponse> {
    let file_path = get_file_path(file_key);
    remove_file(file_path)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while deleting the file")?;
    Ok(())
}

pub fn retrieve_file_from_fs(file_key: String) -> CustomResult<Vec<u8>, errors::ApiErrorResponse> {
    let mut received_data: Vec<u8> = Vec::new();
    let file_path = get_file_path(file_key);
    let mut file = File::open(file_path)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while opening the file")?;
    file.read_to_end(&mut received_data)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while reading the file")?;
    Ok(received_data)
}
