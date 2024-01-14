//!
//! Module for local file system storage operations
//!

use std::{
    fmt::{Display, Formatter},
    fs::{remove_file, File},
    io::{Read, Write},
    path::PathBuf,
};

use error_stack::{IntoReport, ResultExt};
use router_env::env;

use crate::errors::CustomResult;

/// Constructs the file path for a given file key within the file system.
/// The file path is generated based on the workspace path and the provided file key.
pub fn get_file_path(file_key: String) -> PathBuf {
    let mut file_path = PathBuf::new();
    file_path.push(env::workspace_path());
    file_path.push("files");
    file_path.push(file_key);
    file_path
}

/// Represents a file system for storing and managing files locally.
#[derive(Debug, Clone)]
pub struct FileSystem;

impl FileSystem {
    /// Saves the provided file data to the file system under the specified file key.
    pub fn save_file_to_fs(
        &self,
        file_key: String,
        file_data: Vec<u8>,
    ) -> CustomResult<(), FileSystemStorageError> {
        let file_path = get_file_path(file_key);
        let mut file = File::create(file_path)
            .into_report()
            .change_context(FileSystemStorageError("Failed to create file"))?;
        file.write_all(&file_data)
            .into_report()
            .change_context(FileSystemStorageError("Failed while writing into file"))?;
        Ok(())
    }

    /// Deletes the file associated with the specified file key from the file system.
    pub fn delete_file_from_fs(
        &self,
        file_key: String,
    ) -> CustomResult<(), FileSystemStorageError> {
        let file_path = get_file_path(file_key);
        remove_file(file_path)
            .into_report()
            .change_context(FileSystemStorageError("Failed while deleting the file"))?;
        Ok(())
    }

    /// Retrieves the file content associated with the specified file key from the file system.
    pub fn retrieve_file_from_fs(
        &self,
        file_key: String,
    ) -> CustomResult<Vec<u8>, FileSystemStorageError> {
        let mut received_data: Vec<u8> = Vec::new();
        let file_path = get_file_path(file_key);
        let mut file = File::open(file_path)
            .into_report()
            .change_context(FileSystemStorageError("Failed while opening the file"))?;
        file.read_to_end(&mut received_data)
            .into_report()
            .change_context(FileSystemStorageError("Failed while reading the file"))?;
        Ok(received_data)
    }
}

/// Represents an error that can occur during file system storage operations locally.
#[derive(Debug)]
pub struct FileSystemStorageError(&'static str);

impl std::error::Error for FileSystemStorageError {}

impl Display for FileSystemStorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Local file system storage error: {}", self.0)
    }
}
